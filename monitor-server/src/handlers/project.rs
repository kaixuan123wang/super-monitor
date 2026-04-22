//! 项目管理接口。
//!
//! Phase 2 暂未接入认证（Phase 5 补 JWT 中间件），所有请求默认 owner_id = 1。
//! 当 `users`/`groups` 表为空时，会创建一个默认的 owner/group 占位，确保
//! 前端能在 Phase 2 验收阶段正常调试。

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models;
use crate::router::AppState;

fn now_fixed() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap())
}

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
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let mut cursor = models::Project::find();
    if let Some(gid) = q.group_id {
        cursor = cursor.filter(models::project::Column::GroupId.eq(gid));
    }
    if let Some(kw) = q.keyword.as_ref().filter(|s| !s.is_empty()) {
        cursor = cursor.filter(models::project::Column::Name.contains(kw));
    }
    let paginator = cursor
        .order_by_desc(models::project::Column::Id)
        .paginate(db, q.page_size);
    let total = paginator.num_items().await?;
    let items = paginator.fetch_page(q.page.saturating_sub(1)).await?;
    let list: Vec<ProjectDto> = items.into_iter().map(Into::into).collect();

    Ok(Json(json!({
        "code": 0,
        "message": "ok",
        "data": { "list": list, "total": total },
        "pagination": {
            "page": q.page,
            "page_size": q.page_size,
            "total": total,
            "total_pages": (total as f64 / q.page_size as f64).ceil() as u64
        }
    })))
}

pub async fn detail(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let item = models::Project::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(ok(ProjectDto::from(item))))
}

pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<CreateProjectBody>,
) -> AppResult<Json<Value>> {
    if body.name.trim().is_empty() {
        return Err(AppError::BadRequest("project name is required".into()));
    }
    let db = get_db(&state)?;

    let (owner_id, group_id) = ensure_default_owner_group(db, body.group_id).await?;

    let app_id = Uuid::new_v4().simple().to_string();
    let app_key = Uuid::new_v4().simple().to_string()
        + &Uuid::new_v4().simple().to_string().chars().take(32).collect::<String>();

    let active = models::project::ActiveModel {
        id: sea_orm::NotSet,
        name: Set(body.name),
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
    Path(id): Path<i32>,
    Json(body): Json<UpdateProjectBody>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
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
    Path(id): Path<i32>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let res = models::Project::delete_by_id(id).exec(db).await?;
    if res.rows_affected == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(ok(json!({ "deleted": id }))))
}

/// 在 users/groups 表为空时创建占位数据，返回 (owner_id, group_id)。
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
            let u = models::user::ActiveModel {
                id: sea_orm::NotSet,
                username: Set("admin".into()),
                email: Set("admin@local".into()),
                password_hash: Set("!".into()),
                role: Set("super_admin".into()),
                group_id: Set(None),
                avatar: Set(None),
                last_login_at: Set(None),
                created_at: Set(now_fixed()),
                updated_at: Set(now_fixed()),
            };
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

fn get_db(state: &AppState) -> AppResult<&DatabaseConnection> {
    state
        .db
        .as_ref()
        .ok_or_else(|| AppError::Internal("database not connected".into()))
}

fn ok<T: Serialize>(data: T) -> Value {
    json!({ "code": 0, "message": "ok", "data": data })
}
