//! 分组 / 团队管理。

use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
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
    pub owner_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBody {
    pub name: String,
    pub description: Option<String>,
    pub owner_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBody {
    pub name: Option<String>,
    pub description: Option<String>,
    pub owner_id: Option<i32>,
}

pub async fn list(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Value>> {
    // 只有 super_admin 可以列举所有分组
    if current_user.role != "super_admin" {
        return Err(AppError::Forbidden);
    }
    let db = get_db(&state)?;
    let mut cursor = models::Group::find();
    if let Some(keyword) = q.keyword.as_ref().filter(|s| !s.trim().is_empty()) {
        cursor = cursor.filter(models::group::Column::Name.contains(keyword));
    }
    if let Some(owner_id) = q.owner_id {
        cursor = cursor.filter(models::group::Column::OwnerId.eq(owner_id));
    }

    let page_size = q.pagination.safe_page_size();
    let paginator = cursor
        .order_by_desc(models::group::Column::Id)
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
    // super_admin 可查任意分组；其他用户只能查自己所在分组
    let db = get_db(&state)?;
    let group = models::Group::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    if current_user.role != "super_admin" {
        let user = models::User::find_by_id(current_user.id)
            .one(db)
            .await?
            .ok_or(AppError::Unauthorized)?;
        if user.group_id != Some(id) {
            return Err(AppError::Forbidden);
        }
    }
    Ok(Json(json!({ "code": 0, "message": "ok", "data": group })))
}

pub async fn create(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Json(body): Json<CreateBody>,
) -> AppResult<Json<Value>> {
    // 只有 super_admin 可以创建分组
    if current_user.role != "super_admin" {
        return Err(AppError::Forbidden);
    }

    if body.name.trim().is_empty() {
        return Err(AppError::BadRequest("group name is required".into()));
    }
    let db = get_db(&state)?;
    let owner_id = match body.owner_id {
        Some(id) => id,
        None => models::User::find()
            .order_by_asc(models::user::Column::Id)
            .one(db)
            .await?
            .map(|u| u.id)
            .ok_or_else(|| {
                AppError::BadRequest("owner_id is required when no user exists".into())
            })?,
    };
    let group = models::group::ActiveModel {
        id: sea_orm::NotSet,
        name: Set(body.name),
        description: Set(body.description),
        owner_id: Set(owner_id),
        created_at: Set(now_fixed()),
        updated_at: Set(now_fixed()),
    }
    .insert(db)
    .await?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": group })))
}

pub async fn update(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(id): Path<i32>,
    Json(body): Json<UpdateBody>,
) -> AppResult<Json<Value>> {
    // 只有 super_admin 可以更新分组
    if current_user.role != "super_admin" {
        return Err(AppError::Forbidden);
    }

    let db = get_db(&state)?;
    let group = models::Group::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    let mut am: models::group::ActiveModel = group.into();
    if let Some(v) = body.name {
        am.name = Set(v);
    }
    if let Some(v) = body.description {
        am.description = Set(Some(v));
    }
    if let Some(v) = body.owner_id {
        am.owner_id = Set(v);
    }
    am.updated_at = Set(now_fixed());
    let group = am.update(db).await?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": group })))
}

pub async fn remove(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(id): Path<i32>,
) -> AppResult<Json<Value>> {
    // 只有 super_admin 可以删除分组
    if current_user.role != "super_admin" {
        return Err(AppError::Forbidden);
    }

    let db = get_db(&state)?;
    let res = models::Group::delete_by_id(id).exec(db).await?;
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
        assert!(q.owner_id.is_none());
    }

    #[test]
    fn test_list_query_with_filters() {
        let q: ListQuery = serde_json::from_str(r#"{"keyword":"test","owner_id":5}"#).unwrap();
        assert_eq!(q.keyword.as_deref(), Some("test"));
        assert_eq!(q.owner_id, Some(5));
    }

    #[test]
    fn test_create_body() {
        let json_str = r#"{"name":"Engineering","description":"Eng team"}"#;
        let body: CreateBody = serde_json::from_str(json_str).unwrap();
        assert_eq!(body.name, "Engineering");
        assert_eq!(body.description.as_deref(), Some("Eng team"));
        assert!(body.owner_id.is_none());
    }

    #[test]
    fn test_create_body_with_owner() {
        let json_str = r#"{"name":"test","owner_id":1}"#;
        let body: CreateBody = serde_json::from_str(json_str).unwrap();
        assert_eq!(body.owner_id, Some(1));
    }

    #[test]
    fn test_update_body_all_optional() {
        let json_str = r#"{}"#;
        let body: UpdateBody = serde_json::from_str(json_str).unwrap();
        assert!(body.name.is_none());
        assert!(body.description.is_none());
        assert!(body.owner_id.is_none());
    }

    #[test]
    fn test_update_body_partial() {
        let json_str = r#"{"name":"updated"}"#;
        let body: UpdateBody = serde_json::from_str(json_str).unwrap();
        assert_eq!(body.name.as_deref(), Some("updated"));
    }
}
