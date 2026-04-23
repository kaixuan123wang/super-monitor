//! 分组 / 团队管理。

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{FixedOffset, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::models;
use crate::router::AppState;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
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

fn default_page() -> u64 {
    1
}

fn default_page_size() -> u64 {
    20
}

fn now_fixed() -> chrono::DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap())
}

fn get_db(state: &AppState) -> AppResult<&DatabaseConnection> {
    state
        .db
        .as_ref()
        .ok_or_else(|| AppError::Internal("database not connected".into()))
}

pub async fn list(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let mut cursor = models::Group::find();
    if let Some(keyword) = q.keyword.as_ref().filter(|s| !s.trim().is_empty()) {
        cursor = cursor.filter(models::group::Column::Name.contains(keyword));
    }
    if let Some(owner_id) = q.owner_id {
        cursor = cursor.filter(models::group::Column::OwnerId.eq(owner_id));
    }

    let paginator = cursor
        .order_by_desc(models::group::Column::Id)
        .paginate(db, q.page_size);
    let total = paginator.num_items().await?;
    let list = paginator.fetch_page(q.page.saturating_sub(1)).await?;

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

pub async fn detail(State(state): State<AppState>, Path(id): Path<i32>) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let group = models::Group::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": group })))
}

pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<CreateBody>,
) -> AppResult<Json<Value>> {
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
            .ok_or_else(|| AppError::BadRequest("owner_id is required when no user exists".into()))?,
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
    Path(id): Path<i32>,
    Json(body): Json<UpdateBody>,
) -> AppResult<Json<Value>> {
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

pub async fn remove(State(state): State<AppState>, Path(id): Path<i32>) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let res = models::Group::delete_by_id(id).exec(db).await?;
    if res.rows_affected == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(json!({ "code": 0, "message": "ok", "data": { "deleted": id } })))
}
