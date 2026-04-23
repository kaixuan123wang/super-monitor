//! 接口报错查询与统计（Phase 2 完整实现）。

use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

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
    pub url: Option<String>,
    pub method: Option<String>,
    pub status: Option<i32>,
    pub keyword: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub project_id: i32,
    #[serde(default = "default_days")]
    pub days: i64,
}

fn default_page() -> u64 {
    1
}
fn default_page_size() -> u64 {
    20
}
fn default_days() -> i64 {
    7
}

pub async fn list(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    use models::network_error::Column;

    let mut cursor = models::NetworkError::find().filter(Column::ProjectId.eq(q.project_id));
    if let Some(url) = q.url.as_ref().filter(|s| !s.is_empty()) {
        cursor = cursor.filter(Column::Url.contains(url.clone()));
    }
    if let Some(method) = q.method.as_ref().filter(|s| !s.is_empty()) {
        cursor = cursor.filter(Column::Method.eq(method.clone().to_uppercase()));
    }
    if let Some(status) = q.status {
        cursor = cursor.filter(Column::Status.eq(status));
    }
    if let Some(k) = q.keyword.as_ref().filter(|s| !s.is_empty()) {
        cursor = cursor.filter(
            Column::Url
                .contains(k.clone())
                .or(Column::ErrorType.contains(k.clone()))
                .or(Column::RequestBody.contains(k.clone())),
        );
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
    let item = models::NetworkError::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": item })))
}

pub async fn stats(
    State(state): State<AppState>,
    Query(q): Query<StatsQuery>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let since = chrono::Utc::now() - chrono::Duration::days(q.days);
    let since_fixed = since.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap());

    use models::network_error::Column;

    let rows = models::NetworkError::find()
        .filter(Column::ProjectId.eq(q.project_id))
        .filter(Column::CreatedAt.gte(since_fixed))
        .limit(5000)
        .all(db)
        .await?;

    let total = rows.len() as u64;

    // Top URLs
    let mut url_map: HashMap<String, u64> = HashMap::new();
    // Status distribution
    let mut status_map: HashMap<i32, u64> = HashMap::new();
    // Method distribution
    let mut method_map: HashMap<String, u64> = HashMap::new();
    // Duration
    let mut total_duration = 0i64;
    let mut duration_count = 0u64;

    for row in &rows {
        *url_map.entry(row.url.clone()).or_default() += 1;
        if let Some(s) = row.status {
            *status_map.entry(s).or_default() += 1;
        }
        *method_map.entry(row.method.clone().to_uppercase()).or_default() += 1;
        if let Some(d) = row.duration {
            total_duration += d as i64;
            duration_count += 1;
        }
    }

    let mut top_urls: Vec<(String, u64)> = url_map.into_iter().collect();
    top_urls.sort_by(|a, b| b.1.cmp(&a.1));
    top_urls.truncate(10);

    let mut status_distribution: Vec<(i32, u64)> = status_map.into_iter().collect();
    status_distribution.sort_by(|a, b| b.1.cmp(&a.1));

    let mut method_distribution: Vec<(String, u64)> = method_map.into_iter().collect();
    method_distribution.sort_by(|a, b| b.1.cmp(&a.1));

    let avg_duration = if duration_count > 0 {
        (total_duration as f64 / duration_count as f64).round() as i64
    } else {
        0
    };

    Ok(Json(json!({
        "code": 0,
        "message": "ok",
        "data": {
            "total": total,
            "top_urls": top_urls.into_iter().map(|(url, count)| json!({"url": url, "count": count})).collect::<Vec<_>>(),
            "status_distribution": status_distribution.into_iter().map(|(status, count)| json!({"status": status, "count": count})).collect::<Vec<_>>(),
            "method_distribution": method_distribution.into_iter().map(|(method, count)| json!({"method": method, "count": count})).collect::<Vec<_>>(),
            "avg_duration": avg_duration,
        }
    })))
}

fn get_db(state: &AppState) -> AppResult<&DatabaseConnection> {
    state
        .db
        .as_ref()
        .ok_or_else(|| AppError::Internal("database not connected".into()))
}
