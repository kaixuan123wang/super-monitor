//! 项目管理接口。

use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use chrono::{DateTime, FixedOffset};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::middleware::auth::{check_project_access, CurrentUser};
use crate::models;
use crate::router::AppState;
use crate::utils::{get_db, now_fixed};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
    pub group_id: Option<i32>,
    pub keyword: Option<String>,
}

fn default_page() -> u64 {
    1
}
fn default_page_size() -> u64 {
    20
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectBody {
    pub name: String,
    #[serde(default)]
    pub group_id: Option<i32>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub alert_threshold: Option<i32>,
    #[serde(default)]
    pub data_retention_days: Option<i32>,
    #[serde(default)]
    pub environment: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectBody {
    pub name: Option<String>,
    pub description: Option<String>,
    pub alert_threshold: Option<i32>,
    pub alert_webhook: Option<String>,
    pub data_retention_days: Option<i32>,
    pub environment: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProjectDto {
    pub id: i32,
    pub name: String,
    pub app_id: String,
    #[serde(skip_serializing)]
    pub app_key: String,
    pub group_id: i32,
    pub owner_id: i32,
    pub description: Option<String>,
    pub alert_threshold: i32,
    pub alert_webhook: Option<String>,
    pub data_retention_days: i32,
    pub environment: String,
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
}

impl From<models::project::Model> for ProjectDto {
    fn from(m: models::project::Model) -> Self {
        Self {
            id: m.id,
            name: m.name,
            app_id: m.app_id,
            app_key: m.app_key,
            group_id: m.group_id,
            owner_id: m.owner_id,
            description: m.description,
            alert_threshold: m.alert_threshold,
            alert_webhook: m.alert_webhook,
            data_retention_days: m.data_retention_days,
            environment: m.environment,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

pub async fn list(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let mut cursor = models::Project::find();

    // 非 super_admin 只能看自己 owner 或同 group 的项目
    if current_user.role != "super_admin" {
        let user = models::User::find_by_id(current_user.id)
            .one(db)
            .await?
            .ok_or(AppError::Unauthorized)?;
        let mut cond = Condition::any().add(models::project::Column::OwnerId.eq(current_user.id));
        if let Some(gid) = user.group_id {
            cond = cond.add(models::project::Column::GroupId.eq(gid));
        }
        cursor = cursor.filter(cond);
    }

    if let Some(gid) = q.group_id {
        cursor = cursor.filter(models::project::Column::GroupId.eq(gid));
    }
    if let Some(kw) = q.keyword.as_ref().filter(|s| !s.is_empty()) {
        cursor = cursor.filter(models::project::Column::Name.contains(kw));
    }
    let page_size = q.page_size.clamp(1, 100);
    let paginator = cursor
        .order_by_desc(models::project::Column::Id)
        .paginate(db, page_size);
    let total = paginator.num_items().await?;
    let items = paginator.fetch_page(q.page.saturating_sub(1)).await?;
    let list: Vec<ProjectDto> = items.into_iter().map(Into::into).collect();

    Ok(Json(json!({
        "code": 0,
        "message": "ok",
        "data": { "list": list, "total": total },
        "pagination": {
            "page": q.page,
            "page_size": page_size,
            "total": total,
            "total_pages": (total as f64 / page_size as f64).ceil() as u64
        }
    })))
}

pub async fn detail(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i32>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    check_project_access(db, &current_user, id).await?;
    let item = models::Project::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(ok(ProjectDto::from(item))))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(body): Json<CreateProjectBody>,
) -> AppResult<Json<Value>> {
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("project name is required".into()));
    }
    if name.len() > 100 {
        return Err(AppError::BadRequest("project name must be at most 100 characters".into()));
    }
    let db = get_db(&state)?;

    // owner 为当前登录用户；group_id 必须由调用方传入（super_admin 可不传以自动取首个）
    let owner_id = current_user.id;
    let group_id = match body.group_id {
        Some(gid) => {
            let group = models::Group::find_by_id(gid)
                .one(db)
                .await?
                .ok_or_else(|| AppError::BadRequest(format!("group {gid} not found")))?;

            if current_user.role != "super_admin" {
                let user = models::User::find_by_id(current_user.id)
                    .one(db)
                    .await?
                    .ok_or(AppError::Unauthorized)?;
                if user.group_id != Some(group.id) {
                    return Err(AppError::Forbidden);
                }
            }

            group.id
        }
        None => {
            // 尝试取当前用户的 group_id
            let user = models::User::find_by_id(current_user.id)
                .one(db)
                .await?
                .ok_or(AppError::Unauthorized)?;
            match user.group_id {
                Some(gid) => gid,
                None => {
                    // super_admin 且无 group 时自动取第一个 group
                    if current_user.role == "super_admin" {
                        models::Group::find()
                            .order_by_asc(crate::models::group::Column::Id)
                            .one(db)
                            .await?
                            .ok_or_else(|| {
                                AppError::BadRequest(
                                    "no group exists, please create a group first".into(),
                                )
                            })?
                            .id
                    } else {
                        return Err(AppError::BadRequest("group_id is required".into()));
                    }
                }
            }
        }
    };

    let app_id = Uuid::new_v4().simple().to_string();
    let app_key = Uuid::new_v4().simple().to_string()
        + &Uuid::new_v4()
            .simple()
            .to_string()
            .chars()
            .take(32)
            .collect::<String>();

    let active = models::project::ActiveModel {
        id: sea_orm::NotSet,
        name: Set(name),
        app_id: Set(app_id),
        app_key: Set(app_key),
        group_id: Set(group_id),
        owner_id: Set(owner_id),
        description: Set(body.description),
        alert_threshold: Set(body.alert_threshold.unwrap_or(10)),
        alert_webhook: Set(None),
        data_retention_days: Set(body.data_retention_days.unwrap_or(30)),
        environment: Set(body.environment.unwrap_or_else(|| "production".into())),
        created_at: Set(now_fixed()),
        updated_at: Set(now_fixed()),
    };
    let created = active.insert(db).await?;
    Ok(Json(ok(ProjectDto::from(created))))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i32>,
    Json(body): Json<UpdateProjectBody>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    check_project_access(db, &current_user, id).await?;
    let item = models::Project::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    let mut am: models::project::ActiveModel = item.into();
    if let Some(v) = body.name {
        am.name = Set(v);
    }
    if let Some(v) = body.description {
        am.description = Set(Some(v));
    }
    if let Some(v) = body.alert_threshold {
        am.alert_threshold = Set(v);
    }
    if let Some(v) = body.alert_webhook {
        am.alert_webhook = Set(Some(v));
    }
    if let Some(v) = body.data_retention_days {
        am.data_retention_days = Set(v);
    }
    if let Some(v) = body.environment {
        am.environment = Set(v);
    }
    am.updated_at = Set(now_fixed());
    let updated = am.update(db).await?;
    Ok(Json(ok(ProjectDto::from(updated))))
}

pub async fn remove(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i32>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    check_project_access(db, &current_user, id).await?;
    let res = models::Project::delete_by_id(id).exec(db).await?;
    if res.rows_affected == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(ok(json!({ "deleted": id }))))
}

/// 在 users/groups 表为空时创建占位数据，返回 (owner_id, group_id)。
#[allow(dead_code)]
async fn ensure_default_owner_group(
    db: &DatabaseConnection,
    requested_group: Option<i32>,
) -> AppResult<(i32, i32)> {
    use models::group::Column as GroupCol;
    use models::user::Column as UserCol;

    // 先保证至少存在一个 user
    let owner = models::User::find()
        .order_by_asc(UserCol::Id)
        .one(db)
        .await?;
    let owner_id = match owner {
        Some(u) => u.id,
        None => {
            let random_password = Uuid::new_v4().to_string();
            let hash = bcrypt::hash(&random_password, bcrypt::DEFAULT_COST)
                .map_err(|e| AppError::Internal(format!("default admin hash failed: {e}")))?;
            let u = models::user::ActiveModel {
                id: sea_orm::NotSet,
                username: Set("admin".into()),
                email: Set("admin@local".into()),
                password_hash: Set(hash),
                role: Set("super_admin".into()),
                group_id: Set(None),
                avatar: Set(None),
                last_login_at: Set(None),
                created_at: Set(now_fixed()),
                updated_at: Set(now_fixed()),
            };
            tracing::warn!(
                "Created default admin user (username: admin). Initial password printed to stderr."
            );
            eprintln!("\n========================================");
            eprintln!("SECURITY NOTICE: Default admin user created.");
            eprintln!("Username: admin");
            eprintln!("Password: {random_password}");
            eprintln!("Please change this password immediately after first login.");
            eprintln!("========================================\n");
            u.insert(db).await?.id
        }
    };

    let group_id = match requested_group {
        Some(gid) => {
            let exists = models::Group::find_by_id(gid).one(db).await?;
            match exists {
                Some(g) => g.id,
                None => create_default_group(db, owner_id).await?,
            }
        }
        None => {
            let g = models::Group::find()
                .order_by_asc(GroupCol::Id)
                .one(db)
                .await?;
            match g {
                Some(g) => g.id,
                None => create_default_group(db, owner_id).await?,
            }
        }
    };

    Ok((owner_id, group_id))
}

#[allow(dead_code)]
async fn create_default_group(db: &DatabaseConnection, owner_id: i32) -> AppResult<i32> {
    let g = models::group::ActiveModel {
        id: sea_orm::NotSet,
        name: Set("Default".into()),
        description: Set(Some("Auto-created default group".into())),
        owner_id: Set(owner_id),
        created_at: Set(now_fixed()),
        updated_at: Set(now_fixed()),
    };
    Ok(g.insert(db).await?.id)
}

fn ok<T: Serialize>(data: T) -> Value {
    json!({ "code": 0, "message": "ok", "data": data })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_dto_skips_app_key() {
        let dto = ProjectDto {
            id: 1,
            name: "test".into(),
            app_id: "id123".into(),
            app_key: "secret_key".into(),
            group_id: 1,
            owner_id: 1,
            description: None,
            alert_threshold: 10,
            alert_webhook: None,
            data_retention_days: 30,
            environment: "production".into(),
            created_at: now_fixed(),
            updated_at: now_fixed(),
        };
        let json_str = serde_json::to_string(&dto).unwrap();
        assert!(!json_str.contains("secret_key"));
        assert!(json_str.contains("id123"));
    }

    #[test]
    fn test_create_project_body_defaults() {
        let json_str = r#"{"name":"test project"}"#;
        let body: CreateProjectBody = serde_json::from_str(json_str).unwrap();
        assert_eq!(body.name, "test project");
        assert!(body.group_id.is_none());
        assert!(body.description.is_none());
        assert!(body.alert_threshold.is_none());
        assert!(body.data_retention_days.is_none());
        assert!(body.environment.is_none());
    }

    #[test]
    fn test_create_project_body_full() {
        let json_str = r#"{"name":"test","group_id":1,"description":"desc","alert_threshold":5,"data_retention_days":90,"environment":"staging"}"#;
        let body: CreateProjectBody = serde_json::from_str(json_str).unwrap();
        assert_eq!(body.group_id, Some(1));
        assert_eq!(body.alert_threshold, Some(5));
        assert_eq!(body.data_retention_days, Some(90));
        assert_eq!(body.environment.as_deref(), Some("staging"));
    }

    #[test]
    fn test_update_project_body_all_optional() {
        let json_str = r#"{}"#;
        let body: UpdateProjectBody = serde_json::from_str(json_str).unwrap();
        assert!(body.name.is_none());
        assert!(body.description.is_none());
    }

    #[test]
    fn test_list_query_defaults() {
        let q: ListQuery = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(q.page, 1);
        assert_eq!(q.page_size, 20);
        assert!(q.group_id.is_none());
        assert!(q.keyword.is_none());
    }

    #[test]
    fn test_ok_helper() {
        let data = json!({"id": 1});
        let result = ok(data);
        assert_eq!(result["code"], 0);
        assert_eq!(result["message"], "ok");
        assert_eq!(result["data"]["id"], 1);
    }

    #[test]
    fn test_now_fixed_is_utc() {
        let now = now_fixed();
        assert_eq!(now.offset().local_minus_utc(), 0);
    }
}
