//! AI 分析触发 / 结果获取接口。

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{Duration, FixedOffset, Utc};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::models;
use crate::router::AppState;
use crate::services::ai_service;

fn get_db(state: &AppState) -> AppResult<&sea_orm::DatabaseConnection> {
    state
        .db
        .as_ref()
        .ok_or_else(|| AppError::Internal("database not connected".into()))
}

async fn ensure_project_rate_limit(
    db: &sea_orm::DatabaseConnection,
    project_id: i32,
) -> AppResult<()> {
    let since =
        (Utc::now() - Duration::minutes(1)).with_timezone(&FixedOffset::east_opt(0).unwrap());
    let recent_count = models::AiAnalysis::find()
        .filter(models::ai_analysis::Column::ProjectId.eq(project_id))
        .filter(models::ai_analysis::Column::CreatedAt.gte(since))
        .count(db)
        .await?;

    if recent_count >= 20 {
        return Err(AppError::TooManyRequests("分析队列已满，请稍后重试".into()));
    }
    Ok(())
}

// ── 触发 AI 分析 ──────────────────────────────────────────────────────────────

pub async fn trigger(
    State(state): State<AppState>,
    Path(error_id): Path<i64>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;

    let error = models::JsError::find_by_id(error_id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;

    ensure_project_rate_limit(db, error.project_id).await?;

    // 检查是否已有 pending/success 记录
    let existing = models::AiAnalysis::find()
        .filter(models::ai_analysis::Column::ErrorId.eq(error_id))
        .filter(models::ai_analysis::Column::Status.ne("failed"))
        .one(db)
        .await?;

    if let Some(row) = existing {
        return Ok(Json(json!({
            "code": 0, "message": "ok",
            "data": { "task_id": row.id, "status": row.status }
        })));
    }

    // 异步触发
    let db_clone = db.clone();
    let cfg_clone = state.config.clone();
    let error_clone = error.clone();
    tokio::spawn(async move {
        if let Err(e) = ai_service::analyze_error(&db_clone, &cfg_clone, &error_clone).await {
            tracing::warn!(error_id = error_id, error = %e, "AI analysis failed");
        }
    });

    Ok(Json(json!({
        "code": 0, "message": "ok",
        "data": { "task_id": 0, "status": "queued" }
    })))
}

// ── 获取结果 ──────────────────────────────────────────────────────────────────

pub async fn get_result(
    State(state): State<AppState>,
    Path(error_id): Path<i64>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;

    let row = models::AiAnalysis::find()
        .filter(models::ai_analysis::Column::ErrorId.eq(error_id))
        .order_by_desc(models::ai_analysis::Column::CreatedAt)
        .one(db)
        .await?;

    match row {
        Some(r) => Ok(Json(json!({ "code": 0, "message": "ok", "data": r }))),
        None => Ok(Json(
            json!({ "code": 0, "message": "ok", "data": { "status": "not_found" } }),
        )),
    }
}

// ── 历史列表 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub project_id: i32,
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
    pub model_used: Option<String>,
    pub has_suggestion: Option<bool>,
}

fn default_page() -> u64 {
    1
}
fn default_page_size() -> u64 {
    20
}

pub async fn list_analyses(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;

    let mut query =
        models::AiAnalysis::find().filter(models::ai_analysis::Column::ProjectId.eq(q.project_id));

    if let Some(model) = &q.model_used {
        query = query.filter(models::ai_analysis::Column::ModelUsed.eq(model.as_str()));
    }
    if let Some(true) = q.has_suggestion {
        query = query.filter(models::ai_analysis::Column::AiSuggestion.is_not_null());
    }

    let total = query.clone().count(db).await?;
    let items = query
        .order_by_desc(models::ai_analysis::Column::CreatedAt)
        .paginate(db, q.page_size)
        .fetch_page(q.page.saturating_sub(1))
        .await?;

    Ok(Json(json!({
        "code": 0, "message": "ok",
        "data": { "list": items, "total": total }
    })))
}

// ── 批量触发 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct BatchBody {
    pub fingerprint: String,
    pub project_id: i32,
}

pub async fn trigger_batch(
    State(state): State<AppState>,
    Json(body): Json<BatchBody>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;

    let errors = models::JsError::find()
        .filter(models::js_error::Column::ProjectId.eq(body.project_id))
        .filter(models::js_error::Column::Fingerprint.eq(&body.fingerprint))
        .all(db)
        .await?;

    ensure_project_rate_limit(db, body.project_id).await?;

    let queued = errors.len();
    let db_clone = db.clone();
    let cfg_clone = state.config.clone();
    tokio::spawn(async move {
        for error in errors {
            if let Err(e) = ai_service::analyze_error(&db_clone, &cfg_clone, &error).await {
                tracing::warn!(error = %e, "batch AI analysis failed");
            }
        }
    });

    Ok(Json(json!({
        "code": 0, "message": "ok",
        "data": { "queued": queued }
    })))
}
