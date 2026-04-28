//! 告警规则 CRUD + 告警日志查询接口。

use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use chrono::{FixedOffset, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::middleware::auth::{check_project_access, CurrentUser};
use crate::models;
use crate::router::AppState;
use crate::services::alert_service;

fn now_fixed() -> chrono::DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap())
}

fn get_db(state: &AppState) -> AppResult<&sea_orm::DatabaseConnection> {
    state
        .db
        .as_ref()
        .ok_or_else(|| AppError::Internal("database not connected".into()))
}

// ── 规则列表 ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RulesQuery {
    pub project_id: i32,
}

pub async fn list_rules(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Query(q): Query<RulesQuery>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    check_project_access(db, &current_user, q.project_id).await?;
    let items = models::AlertRule::find()
        .filter(models::alert_rule::Column::ProjectId.eq(q.project_id))
        .order_by_desc(models::alert_rule::Column::CreatedAt)
        .limit(200)
        .all(db)
        .await?;

    Ok(Json(json!({ "code": 0, "message": "ok", "data": { "list": items, "total": items.len() } })))
}

// ── 创建规则 ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateRuleBody {
    pub project_id: i32,
    pub name: String,
    pub rule_type: String,
    pub threshold: Option<i32>,
    #[serde(default = "default_interval")]
    pub interval_minutes: i32,
    pub webhook_url: Option<String>,
    pub email: Option<String>,
}

fn default_interval() -> i32 {
    60
}

pub async fn create_rule(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(body): Json<CreateRuleBody>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    check_project_access(db, &current_user, body.project_id).await?;

    let valid_types = ["error_spike", "failure_rate", "new_error", "p0_error", "error_trend"];
    if !valid_types.contains(&body.rule_type.as_str()) {
        return Err(AppError::BadRequest(format!("invalid rule_type: {}", body.rule_type)));
    }
    if !(1..=1440).contains(&body.interval_minutes) {
        return Err(AppError::BadRequest("interval_minutes must be between 1 and 1440".into()));
    }
    if let Some(url) = body.webhook_url.as_deref().filter(|s| !s.trim().is_empty()) {
        alert_service::validate_webhook_url(url)
            .map_err(|e| AppError::BadRequest(format!("invalid webhook_url: {e}")))?;
    }

    let am = models::alert_rule::ActiveModel {
        id: sea_orm::NotSet,
        project_id: Set(body.project_id),
        name: Set(body.name),
        rule_type: Set(body.rule_type),
        threshold: Set(body.threshold),
        interval_minutes: Set(body.interval_minutes),
        is_enabled: Set(true),
        webhook_url: Set(body.webhook_url),
        email: Set(body.email),
        created_at: Set(now_fixed()),
        updated_at: Set(now_fixed()),
    };
    let saved = am.insert(db).await?;

    Ok(Json(json!({ "code": 0, "message": "ok", "data": saved })))
}

// ── 更新规则 ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct UpdateRuleBody {
    pub name: Option<String>,
    pub threshold: Option<i32>,
    pub interval_minutes: Option<i32>,
    pub is_enabled: Option<bool>,
    pub webhook_url: Option<String>,
    pub email: Option<String>,
}

pub async fn update_rule(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i32>,
    Json(body): Json<UpdateRuleBody>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let row = models::AlertRule::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    check_project_access(db, &current_user, row.project_id).await?;

    let mut am: models::alert_rule::ActiveModel = row.into();
    if let Some(v) = body.name {
        am.name = Set(v);
    }
    if let Some(v) = body.threshold {
        am.threshold = Set(Some(v));
    }
    if let Some(v) = body.interval_minutes {
        if !(1..=1440).contains(&v) {
            return Err(AppError::BadRequest("interval_minutes must be between 1 and 1440".into()));
        }
        am.interval_minutes = Set(v);
    }
    if let Some(v) = body.is_enabled {
        am.is_enabled = Set(v);
    }
    if let Some(v) = body.webhook_url {
        if !v.trim().is_empty() {
            alert_service::validate_webhook_url(&v)
                .map_err(|e| AppError::BadRequest(format!("invalid webhook_url: {e}")))?;
        }
        am.webhook_url = Set(Some(v));
    }
    if let Some(v) = body.email {
        am.email = Set(Some(v));
    }
    am.updated_at = Set(now_fixed());

    let updated = am.update(db).await?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": updated })))
}

// ── 删除规则 ─────────────────────────────────────────────────────────────────

pub async fn delete_rule(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i32>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let row = models::AlertRule::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    check_project_access(db, &current_user, row.project_id).await?;
    models::AlertRule::delete_by_id(id).exec(db).await?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": { "deleted": 1 } })))
}

// ── 告警日志列表 ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    pub project_id: i32,
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
    pub status: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

fn default_page() -> u64 {
    1
}
fn default_page_size() -> u64 {
    20
}

pub async fn list_logs(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Query(q): Query<LogsQuery>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    check_project_access(db, &current_user, q.project_id).await?;

    let mut query =
        models::AlertLog::find().filter(models::alert_log::Column::ProjectId.eq(q.project_id));

    if let Some(s) = &q.status {
        query = query.filter(models::alert_log::Column::Status.eq(s.as_str()));
    }

    let page_size = q.page_size.clamp(1, 100);
    let total = query.clone().count(db).await?;
    let items = query
        .order_by_desc(models::alert_log::Column::CreatedAt)
        .paginate(db, page_size)
        .fetch_page(q.page.saturating_sub(1))
        .await?;

    Ok(Json(json!({ "code": 0, "message": "ok", "data": { "list": items, "total": total } })))
}

// ── 告警日志详情 ──────────────────────────────────────────────────────────────

pub async fn log_detail(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i32>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let row = models::AlertLog::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    check_project_access(db, &current_user, row.project_id).await?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": row })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_rule_body_deserialize() {
        let json_str =
            r#"{"project_id":1,"name":"Error Spike","rule_type":"error_spike","threshold":10}"#;
        let body: CreateRuleBody = serde_json::from_str(json_str).unwrap();
        assert_eq!(body.project_id, 1);
        assert_eq!(body.name, "Error Spike");
        assert_eq!(body.rule_type, "error_spike");
        assert_eq!(body.threshold, Some(10));
        assert_eq!(body.interval_minutes, 60);
    }

    #[test]
    fn test_create_rule_body_custom_interval() {
        let json_str =
            r#"{"project_id":1,"name":"test","rule_type":"failure_rate","interval_minutes":30}"#;
        let body: CreateRuleBody = serde_json::from_str(json_str).unwrap();
        assert_eq!(body.interval_minutes, 30);
    }

    #[test]
    fn test_create_rule_body_serialization() {
        let body = CreateRuleBody {
            project_id: 1,
            name: "test".into(),
            rule_type: "error_spike".into(),
            threshold: Some(5),
            interval_minutes: 60,
            webhook_url: Some("https://hook.example.com".into()),
            email: None,
        };
        let json_str = serde_json::to_string(&body).unwrap();
        assert!(json_str.contains("error_spike"));
        assert!(json_str.contains("hook.example.com"));
    }

    #[test]
    fn test_update_rule_body_all_optional() {
        let json_str = r#"{}"#;
        let body: UpdateRuleBody = serde_json::from_str(json_str).unwrap();
        assert!(body.name.is_none());
        assert!(body.threshold.is_none());
        assert!(body.interval_minutes.is_none());
        assert!(body.is_enabled.is_none());
    }

    #[test]
    fn test_update_rule_body_partial() {
        let json_str = r#"{"name":"updated","is_enabled":false}"#;
        let body: UpdateRuleBody = serde_json::from_str(json_str).unwrap();
        assert_eq!(body.name.as_deref(), Some("updated"));
        assert_eq!(body.is_enabled, Some(false));
    }

    #[test]
    fn test_rules_query_deserialize() {
        let json_str = r#"{"project_id":1}"#;
        let q: RulesQuery = serde_json::from_str(json_str).unwrap();
        assert_eq!(q.project_id, 1);
    }

    #[test]
    fn test_logs_query_defaults() {
        let json_str = r#"{"project_id":1}"#;
        let q: LogsQuery = serde_json::from_str(json_str).unwrap();
        assert_eq!(q.project_id, 1);
        assert_eq!(q.page, 1);
        assert_eq!(q.page_size, 20);
        assert!(q.status.is_none());
    }

    #[test]
    fn test_now_fixed_is_utc() {
        let now = now_fixed();
        assert_eq!(now.offset().local_minus_utc(), 0);
    }
}
