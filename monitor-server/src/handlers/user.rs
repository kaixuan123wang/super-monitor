//! 用户管理。

use axum::{
    extract::{Path, Query, State},
    Json,
};
use bcrypt::{hash, DEFAULT_COST};
use chrono::{FixedOffset, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, Set,
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

    let paginator = cursor
        .order_by_desc(models::user::Column::Id)
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
    let user = models::User::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": user })))
}

pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<CreateBody>,
) -> AppResult<Json<Value>> {
    if body.username.trim().is_empty() || body.email.trim().is_empty() || body.password.len() < 6 {
        return Err(AppError::BadRequest(
            "username/email are required and password must be at least 6 chars".into(),
        ));
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
    Path(id): Path<i32>,
    Json(body): Json<UpdateBody>,
) -> AppResult<Json<Value>> {
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
        if v.len() < 6 {
            return Err(AppError::BadRequest(
                "password must be at least 6 chars".into(),
            ));
        }
        am.password_hash = Set(hash(v, DEFAULT_COST)
            .map_err(|e| AppError::Internal(format!("password hash failed: {e}")))?);
    }
    if let Some(v) = body.role {
        am.role = Set(v);
    }
    if body.group_id.is_some() {
        am.group_id = Set(body.group_id);
    }
    if body.avatar.is_some() {
        am.avatar = Set(body.avatar);
    }
    am.updated_at = Set(now_fixed());
    let user = am.update(db).await?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": user })))
}

pub async fn remove(State(state): State<AppState>, Path(id): Path<i32>) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let res = models::User::delete_by_id(id).exec(db).await?;
    if res.rows_affected == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(json!({ "code": 0, "message": "ok", "data": { "deleted": id } })))
}
