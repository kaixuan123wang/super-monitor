//! 告警规则 CRUD + 告警日志查询接口。

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{FixedOffset, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::models;
use crate::router::AppState;

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
    Query(q): Query<RulesQuery>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let items = models::AlertRule::find()
        .filter(models::alert_rule::Column::ProjectId.eq(q.project_id))
        .order_by_desc(models::alert_rule::Column::CreatedAt)
        .all(db)
        .await?;

    Ok(Json(
        json!({ "code": 0, "message": "ok", "data": { "list": items, "total": items.len() } }),
    ))
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
    Json(body): Json<CreateRuleBody>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;

    let valid_types = [
        "error_spike",
        "failure_rate",
        "new_error",
        "p0_error",
        "error_trend",
    ];
    if !valid_types.contains(&body.rule_type.as_str()) {
        return Err(AppError::BadRequest(format!(
            "invalid rule_type: {}",
            body.rule_type
        )));
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
    Path(id): Path<i32>,
    Json(body): Json<UpdateRuleBody>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let row = models::AlertRule::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;

    let mut am: models::alert_rule::ActiveModel = row.into();
    if let Some(v) = body.name {
        am.name = Set(v);
    }
    if let Some(v) = body.threshold {
        am.threshold = Set(Some(v));
    }
    if let Some(v) = body.interval_minutes {
        am.interval_minutes = Set(v);
    }
    if let Some(v) = body.is_enabled {
        am.is_enabled = Set(v);
    }
    if let Some(v) = body.webhook_url {
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
    Path(id): Path<i32>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    models::AlertRule::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    models::AlertRule::delete_by_id(id).exec(db).await?;
    Ok(Json(
        json!({ "code": 0, "message": "ok", "data": { "deleted": 1 } }),
    ))
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
    Query(q): Query<LogsQuery>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;

    let mut query =
        models::AlertLog::find().filter(models::alert_log::Column::ProjectId.eq(q.project_id));

    if let Some(s) = &q.status {
        query = query.filter(models::alert_log::Column::Status.eq(s.as_str()));
    }

    let total = query.clone().count(db).await?;
    let items = query
        .order_by_desc(models::alert_log::Column::CreatedAt)
        .paginate(db, q.page_size)
        .fetch_page(q.page.saturating_sub(1))
        .await?;

    Ok(Json(
        json!({ "code": 0, "message": "ok", "data": { "list": items, "total": total } }),
    ))
}

// ── 告警日志详情 ──────────────────────────────────────────────────────────────

pub async fn log_detail(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let row = models::AlertLog::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": row })))
}
