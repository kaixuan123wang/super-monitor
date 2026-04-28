//! 用户管理。

use axum::{
    extract::{Path, Query, State},
    Json,
};
use bcrypt::{hash, DEFAULT_COST};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    Set,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::middleware::auth::CurrentUser;
use crate::models;
use crate::router::AppState;
use crate::utils::{get_db, now_fixed, PaginationParams};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    pub keyword: Option<String>,
    pub group_id: Option<i32>,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBody {
    pub username: String,
    pub email: String,
    pub password: String,
    pub role: Option<String>,
    pub group_id: Option<i32>,
    pub avatar: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBody {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub role: Option<String>,
    pub group_id: Option<i32>,
    pub avatar: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Value>> {
    // 只有 super_admin 可以列举所有用户
    if current_user.role != "super_admin" {
        return Err(AppError::Forbidden);
    }
    let db = get_db(&state)?;
    let mut cursor = models::User::find();
    if let Some(keyword) = q.keyword.as_ref().filter(|s| !s.trim().is_empty()) {
        cursor = cursor.filter(
            Condition::any()
                .add(models::user::Column::Username.contains(keyword))
                .add(models::user::Column::Email.contains(keyword)),
        );
    }
    if let Some(group_id) = q.group_id {
        cursor = cursor.filter(models::user::Column::GroupId.eq(group_id));
    }
    if let Some(role) = q.role.as_ref().filter(|s| !s.trim().is_empty()) {
        cursor = cursor.filter(models::user::Column::Role.eq(role.as_str()));
    }

    let page_size = q.pagination.safe_page_size();
    let paginator = cursor
        .order_by_desc(models::user::Column::Id)
        .paginate(db, page_size);
    let total = paginator.num_items().await?;
    let list = paginator
        .fetch_page(q.pagination.page.saturating_sub(1))
        .await?;

    Ok(Json(crate::utils::paginated_response(list, total, q.pagination.page, page_size)))
}

pub async fn detail(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(id): Path<i32>,
) -> AppResult<Json<Value>> {
    // super_admin 可查任意用户；普通用户只能查自己
    if current_user.role != "super_admin" && current_user.id != id {
        return Err(AppError::Forbidden);
    }
    let db = get_db(&state)?;
    let user = models::User::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": user })))
}

pub async fn create(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Json(body): Json<CreateBody>,
) -> AppResult<Json<Value>> {
    // 只有 super_admin 可以创建用户
    if current_user.role != "super_admin" {
        return Err(AppError::Forbidden);
    }

    if body.username.trim().is_empty() || body.email.trim().is_empty() || body.password.len() < 12 {
        return Err(AppError::BadRequest(
            "username/email are required and password must be at least 12 chars".into(),
        ));
    }
    if body.username.trim().len() > 64 {
        return Err(AppError::BadRequest("username must be at most 64 characters".into()));
    }
    if body.email.trim().len() > 254 {
        return Err(AppError::BadRequest("email must be at most 254 characters".into()));
    }
    let db = get_db(&state)?;
    let password_hash = hash(&body.password, DEFAULT_COST)
        .map_err(|e| AppError::Internal(format!("password hash failed: {e}")))?;
    let user = models::user::ActiveModel {
        id: sea_orm::NotSet,
        username: Set(body.username),
        email: Set(body.email),
        password_hash: Set(password_hash),
        role: Set(body.role.unwrap_or_else(|| "member".into())),
        group_id: Set(body.group_id),
        avatar: Set(body.avatar),
        last_login_at: Set(None),
        created_at: Set(now_fixed()),
        updated_at: Set(now_fixed()),
    }
    .insert(db)
    .await?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": user })))
}

pub async fn update(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(id): Path<i32>,
    Json(body): Json<UpdateBody>,
) -> AppResult<Json<Value>> {
    // 只有 super_admin 可以更新用户，或者用户更新自己
    if current_user.role != "super_admin" && current_user.id != id {
        return Err(AppError::Forbidden);
    }

    // 非 super_admin 不能修改角色
    if current_user.role != "super_admin" && body.role.is_some() {
        return Err(AppError::Forbidden);
    }
    // 非 super_admin 不能自行切换 group，否则会扩大项目访问范围
    if current_user.role != "super_admin" && body.group_id.is_some() {
        return Err(AppError::Forbidden);
    }

    let db = get_db(&state)?;
    let user = models::User::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    let mut am: models::user::ActiveModel = user.into();
    if let Some(v) = body.username {
        am.username = Set(v);
    }
    if let Some(v) = body.email {
        am.email = Set(v);
    }
    if let Some(v) = body.password {
        if v.len() < 12 {
            return Err(AppError::BadRequest("password must be at least 12 chars".into()));
        }
        am.password_hash = Set(hash(v, DEFAULT_COST)
            .map_err(|e| AppError::Internal(format!("password hash failed: {e}")))?);
    }
    if let Some(v) = body.role {
        am.role = Set(v);
    }
    if let Some(v) = body.group_id {
        am.group_id = Set(Some(v));
    }
    if let Some(v) = body.avatar {
        am.avatar = Set(Some(v));
    }
    am.updated_at = Set(now_fixed());
    let user = am.update(db).await?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": user })))
}

pub async fn remove(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(id): Path<i32>,
) -> AppResult<Json<Value>> {
    // 只有 super_admin 可以删除用户
    if current_user.role != "super_admin" {
        return Err(AppError::Forbidden);
    }

    // 不能删除自己
    if current_user.id == id {
        return Err(AppError::BadRequest("cannot delete yourself".into()));
    }

    let db = get_db(&state)?;
    let res = models::User::delete_by_id(id).exec(db).await?;
    if res.rows_affected == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(json!({ "code": 0, "message": "ok", "data": { "deleted": id } })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_query_defaults() {
        let q: ListQuery = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(q.pagination.page, 1);
        assert_eq!(q.pagination.page_size, 20);
        assert!(q.keyword.is_none());
        assert!(q.group_id.is_none());
        assert!(q.role.is_none());
    }

    #[test]
    fn test_list_query_with_filters() {
        let q: ListQuery =
            serde_json::from_str(r#"{"keyword":"admin","group_id":1,"role":"super_admin"}"#)
                .unwrap();
        assert_eq!(q.keyword.as_deref(), Some("admin"));
        assert_eq!(q.group_id, Some(1));
        assert_eq!(q.role.as_deref(), Some("super_admin"));
    }

    #[test]
    fn test_create_body() {
        let json_str = r#"{"username":"test","email":"test@test.com","password":"123456"}"#;
        let body: CreateBody = serde_json::from_str(json_str).unwrap();
        assert_eq!(body.username, "test");
        assert_eq!(body.email, "test@test.com");
        assert_eq!(body.password, "123456");
        assert!(body.role.is_none());
        assert!(body.group_id.is_none());
    }

    #[test]
    fn test_create_body_with_optional_fields() {
        let json_str = r#"{"username":"test","email":"t@t.com","password":"123456","role":"admin","group_id":1,"avatar":"http://img.png"}"#;
        let body: CreateBody = serde_json::from_str(json_str).unwrap();
        assert_eq!(body.role.as_deref(), Some("admin"));
        assert_eq!(body.group_id, Some(1));
        assert_eq!(body.avatar.as_deref(), Some("http://img.png"));
    }

    #[test]
    fn test_update_body_all_optional() {
        let json_str = r#"{}"#;
        let body: UpdateBody = serde_json::from_str(json_str).unwrap();
        assert!(body.username.is_none());
        assert!(body.email.is_none());
        assert!(body.password.is_none());
        assert!(body.role.is_none());
        assert!(body.group_id.is_none());
        assert!(body.avatar.is_none());
    }

    #[test]
    fn test_update_body_partial() {
        let json_str = r#"{"username":"new_name","avatar":"http://new.png"}"#;
        let body: UpdateBody = serde_json::from_str(json_str).unwrap();
        assert_eq!(body.username.as_deref(), Some("new_name"));
        assert_eq!(body.avatar.as_deref(), Some("http://new.png"));
    }
}
