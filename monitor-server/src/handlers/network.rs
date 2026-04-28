//! 接口报错查询与统计（Phase 2 完整实现）。

use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::middleware::auth::{check_project_access, CurrentUser};
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
    Extension(current_user): Extension<CurrentUser>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    check_project_access(db, &current_user, q.project_id).await?;
    use models::network_error::Column;

    let mut cursor = models::NetworkError::find().filter(Column::ProjectId.eq(q.project_id));
    if let Some(url) = q.url.as_ref().filter(|s| !s.is_empty()) {
        cursor = cursor.filter(Column::Url.contains(url.as_str()));
    }
    if let Some(method) = q.method.as_ref().filter(|s| !s.is_empty()) {
        cursor = cursor.filter(Column::Method.eq(method.to_uppercase()));
    }
    if let Some(status) = q.status {
        cursor = cursor.filter(Column::Status.eq(status));
    }
    if let Some(k) = q.keyword.as_ref().filter(|s| !s.is_empty()) {
        cursor = cursor.filter(
            Column::Url
                .contains(k.as_str())
                .or(Column::ErrorType.contains(k.as_str()))
                .or(Column::RequestBody.contains(k.as_str())),
        );
    }

    let page_size = q.page_size.clamp(1, 100);
    let paginator = cursor
        .order_by_desc(Column::CreatedAt)
        .paginate(db, page_size);
    let total = paginator.num_items().await?;
    let items = paginator.fetch_page(q.page.saturating_sub(1)).await?;

    Ok(Json(json!({
        "code": 0,
        "message": "ok",
        "data": { "list": items, "total": total },
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
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let item = models::NetworkError::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    check_project_access(db, &current_user, item.project_id).await?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": item })))
}

pub async fn stats(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Query(q): Query<StatsQuery>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    check_project_access(db, &current_user, q.project_id).await?;
    let days = q.days.clamp(1, 90);
    let since = chrono::Utc::now() - chrono::Duration::days(days);
    let since_fixed = since.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap());

    use sea_orm::{ConnectionTrait, Statement};

    // 总数
    let total: i64 = models::NetworkError::find()
        .filter(models::network_error::Column::ProjectId.eq(q.project_id))
        .filter(models::network_error::Column::CreatedAt.gte(since_fixed))
        .count(db)
        .await? as i64;

    // Top URLs — SQL GROUP BY
    let top_urls_rows = db
        .query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT url, COUNT(*) AS cnt FROM network_errors \
             WHERE project_id = $1 AND created_at >= $2 \
             GROUP BY url ORDER BY cnt DESC LIMIT 10",
            [q.project_id.into(), since_fixed.into()],
        ))
        .await?;
    let top_urls: Vec<Value> = top_urls_rows
        .into_iter()
        .map(|r| {
            let url: String = r.try_get("", "url").unwrap_or_default();
            let cnt: i64 = r.try_get("", "cnt").unwrap_or(0);
            json!({"url": url, "count": cnt})
        })
        .collect();

    // Status 分布 — SQL GROUP BY
    let status_rows = db
        .query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT status, COUNT(*) AS cnt FROM network_errors \
             WHERE project_id = $1 AND created_at >= $2 AND status IS NOT NULL \
             GROUP BY status ORDER BY cnt DESC",
            [q.project_id.into(), since_fixed.into()],
        ))
        .await?;
    let status_distribution: Vec<Value> = status_rows
        .into_iter()
        .map(|r| {
            let status: i32 = r.try_get("", "status").unwrap_or(0);
            let cnt: i64 = r.try_get("", "cnt").unwrap_or(0);
            json!({"status": status, "count": cnt})
        })
        .collect();

    // Method 分布 — SQL GROUP BY
    let method_rows = db
        .query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT UPPER(method) AS method, COUNT(*) AS cnt FROM network_errors \
             WHERE project_id = $1 AND created_at >= $2 \
             GROUP BY UPPER(method) ORDER BY cnt DESC",
            [q.project_id.into(), since_fixed.into()],
        ))
        .await?;
    let method_distribution: Vec<Value> = method_rows
        .into_iter()
        .map(|r| {
            let method: String = r.try_get("", "method").unwrap_or_else(|_| "GET".into());
            let cnt: i64 = r.try_get("", "cnt").unwrap_or(0);
            json!({"method": method, "count": cnt})
        })
        .collect();

    // 平均耗时 — SQL AVG
    let avg_row = db
        .query_one(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT AVG(duration) AS avg_dur FROM network_errors \
             WHERE project_id = $1 AND created_at >= $2 AND duration IS NOT NULL",
            [q.project_id.into(), since_fixed.into()],
        ))
        .await?;
    let avg_duration: i64 = avg_row
        .and_then(|r| r.try_get::<f64>("", "avg_dur").ok())
        .map(|v| v.round() as i64)
        .unwrap_or(0);

    Ok(Json(json!({
        "code": 0,
        "message": "ok",
        "data": {
            "total": total,
            "top_urls": top_urls,
            "status_distribution": status_distribution,
            "method_distribution": method_distribution,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_query_defaults() {
        let q: ListQuery = serde_json::from_str(r#"{"project_id":1}"#).unwrap();
        assert_eq!(q.project_id, 1);
        assert_eq!(q.page, 1);
        assert_eq!(q.page_size, 20);
        assert!(q.url.is_none());
        assert!(q.method.is_none());
        assert!(q.status.is_none());
    }

    #[test]
    fn test_list_query_with_filters() {
        let q: ListQuery =
            serde_json::from_str(r#"{"project_id":1,"url":"api","method":"POST","status":500}"#)
                .unwrap();
        assert_eq!(q.url.as_deref(), Some("api"));
        assert_eq!(q.method.as_deref(), Some("POST"));
        assert_eq!(q.status, Some(500));
    }

    #[test]
    fn test_stats_query_defaults() {
        let q: StatsQuery = serde_json::from_str(r#"{"project_id":1}"#).unwrap();
        assert_eq!(q.project_id, 1);
        assert_eq!(q.days, 7);
    }

    #[test]
    fn test_stats_query_custom_days() {
        let q: StatsQuery = serde_json::from_str(r#"{"project_id":1,"days":30}"#).unwrap();
        assert_eq!(q.days, 30);
    }
}
