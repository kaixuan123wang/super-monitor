//! 错误数据查询 / 统计接口。
//!
//! Phase 2 实现列表 + 详情。聚合/趋势/相似错误留到 Phase 3+。

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
    Extension(current_user): Extension<CurrentUser>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    check_project_access(db, &current_user, q.project_id).await?;
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
    let item = models::JsError::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    check_project_access(db, &current_user, item.project_id).await?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": item })))
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
        assert!(q.error_type.is_none());
        assert!(q.fingerprint.is_none());
        assert!(q.browser.is_none());
        assert!(q.os.is_none());
        assert!(q.release.is_none());
        assert!(q.environment.is_none());
        assert!(q.keyword.is_none());
    }

    #[test]
    fn test_list_query_with_filters() {
        let q: ListQuery = serde_json::from_str(
            r#"{"project_id":1,"error_type":"TypeError","browser":"Chrome","os":"Windows"}"#,
        )
        .unwrap();
        assert_eq!(q.error_type.as_deref(), Some("TypeError"));
        assert_eq!(q.browser.as_deref(), Some("Chrome"));
        assert_eq!(q.os.as_deref(), Some("Windows"));
    }

    #[test]
    fn test_list_query_with_keyword() {
        let q: ListQuery =
            serde_json::from_str(r#"{"project_id":1,"keyword":"undefined"}"#).unwrap();
        assert_eq!(q.keyword.as_deref(), Some("undefined"));
    }
}
