//! 错误数据查询 / 统计接口。
//!
//! Phase 2 实现列表 + 详情。聚合/趋势/相似错误留到 Phase 3+。

use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::models;
use crate::router::AppState;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub project_id: i32,
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
    pub error_type: Option<String>,
    pub fingerprint: Option<String>,
    pub browser: Option<String>,
    pub os: Option<String>,
    pub release: Option<String>,
    pub environment: Option<String>,
    pub keyword: Option<String>,
}

fn default_page() -> u64 {
    1
}
fn default_page_size() -> u64 {
    20
}

pub async fn list(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    use models::js_error::Column;

    let mut cursor = models::JsError::find().filter(Column::ProjectId.eq(q.project_id));
    if let Some(t) = q.error_type.as_ref() {
        cursor = cursor.filter(Column::ErrorType.eq(t.clone()));
    }
    if let Some(fp) = q.fingerprint.as_ref() {
        cursor = cursor.filter(Column::Fingerprint.eq(fp.clone()));
    }
    if let Some(b) = q.browser.as_ref() {
        cursor = cursor.filter(Column::Browser.eq(b.clone()));
    }
    if let Some(o) = q.os.as_ref() {
        cursor = cursor.filter(Column::Os.eq(o.clone()));
    }
    if let Some(r) = q.release.as_ref() {
        cursor = cursor.filter(Column::Release.eq(r.clone()));
    }
    if let Some(e) = q.environment.as_ref() {
        cursor = cursor.filter(Column::Environment.eq(e.clone()));
    }
    if let Some(k) = q.keyword.as_ref().filter(|s| !s.is_empty()) {
        cursor = cursor.filter(Column::Message.contains(k));
    }

    let paginator = cursor
        .order_by_desc(Column::CreatedAt)
        .paginate(db, q.page_size);
    let total = paginator.num_items().await?;
    let items = paginator.fetch_page(q.page.saturating_sub(1)).await?;

    Ok(Json(json!({
        "code": 0,
        "message": "ok",
        "data": { "list": items, "total": total },
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
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let item = models::JsError::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": item })))
}

fn get_db(state: &AppState) -> AppResult<&DatabaseConnection> {
    state
        .db
        .as_ref()
        .ok_or_else(|| AppError::Internal("database not connected".into()))
}
