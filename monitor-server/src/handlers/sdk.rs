//! SDK 数据上报接口 `/api/v1/collect`。
//!
//! Phase 2：按 `type` 字段分发到不同服务：
//! - error      → js_errors
//! - network    → network_errors
//! - performance→ performance_data
//! - track / track_batch → track_events
//! - profile    → track_user_profiles
//! - track_signup → track_id_mapping (+ profile merge 占位)
//! - batch      → 递归处理多条 CollectPayload
//! - breadcrumb → 目前仅记录日志，不单独持久化（与错误一并存储）
//!
//! 鉴权：通过 `X-App-Id` 头部查询 `projects` 表，若项目不存在或 app_key 不匹配，拒绝。

use axum::{extract::State, http::HeaderMap, Json};
use chrono::{DateTime, Duration, FixedOffset, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::models;
use crate::router::AppState;
use crate::services::{ai_service, alert_service, identity_service, track_service};

#[derive(Debug, Deserialize)]
pub struct CollectPayload {
    #[serde(rename = "type")]
    pub type_: String,
    pub data: Value,
    #[serde(default)]
    pub context: Option<Value>,
}

pub async fn collect(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> AppResult<Json<Value>> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::Internal("database not connected".into()))?;

    let app_id = header_str(&headers, "x-app-id")?;
    let app_key = header_str(&headers, "x-app-key")?;

    // 鉴权：查询项目
    let project = models::Project::find()
        .filter(models::project::Column::AppId.eq(app_id.clone()))
        .one(db)
        .await?
        .ok_or(AppError::Unauthorized)?;
    if project.app_key != app_key {
        return Err(AppError::Unauthorized);
    }

    dispatch(db, &project, payload, None, &state.alert_tx, &state.config).await?;

    Ok(Json(json!({ "code": 0, "message": "ok", "data": null })))
}

fn header_str(headers: &HeaderMap, name: &str) -> AppResult<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest(format!("missing header: {name}")))
}

/// 按 type 递归分发。
fn dispatch<'a>(
    db: &'a DatabaseConnection,
    project: &'a models::project::Model,
    payload: Value,
    inherited_context: Option<Value>,
    alert_tx: &'a tokio::sync::broadcast::Sender<alert_service::AlertEvent>,
    cfg: &'a Config,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = AppResult<()>> + Send + 'a>> {
    Box::pin(async move {
        let type_ = payload
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let data = payload.get("data").cloned().unwrap_or(Value::Null);
        let context = extract_context(&data, &payload, inherited_context.as_ref());

        match type_.as_str() {
            "error" => save_error(db, project, &data, &context, alert_tx, cfg).await?,
            "network" => save_network(db, project, &data, &context).await?,
            "performance" => save_performance(db, project, &data, &context).await?,
            "track" => {
                track_service::save_track_event(db, project, &data, &context, None).await?;
            }
            "track_batch" => {
                // data 可能是 [{event, ...}, ...]
                if let Some(arr) = data.as_array() {
                    for item in arr {
                        track_service::save_track_event(db, project, item, &context, None).await?;
                    }
                }
            }
            "profile" => {
                track_service::save_profile(db, project, &data).await?;
            }
            "track_signup" => {
                identity_service::save_id_mapping(db, project.id, &data).await?;
            }
            "breadcrumb" => {
                // 面包屑默认仅作为错误上下文，不单独持久化。
                tracing::debug!(?data, "breadcrumb event received");
            }
            "batch" => {
                if let Some(arr) = data.as_array() {
                    for item in arr {
                        dispatch(
                            db,
                            project,
                            item.clone(),
                            Some(context.clone()),
                            alert_tx,
                            cfg,
                        )
                        .await?;
                    }
                }
            }
            other => {
                tracing::warn!(type_ = %other, "unknown collect type, ignored");
            }
        }

        Ok(())
    })
}

/// 从 data 中抽出 `__context` 字段（SDK 注入），否则继承外层 batch context。
fn extract_context(data: &Value, payload: &Value, inherited_context: Option<&Value>) -> Value {
    if let Some(ctx) = data.get("__context") {
        return ctx.clone();
    }
    if let Some(ctx) = payload.get("context") {
        return ctx.clone();
    }
    if let Some(ctx) = inherited_context {
        return ctx.clone();
    }
    Value::Null
}

fn ctx_str(ctx: &Value, key: &str) -> Option<String> {
    ctx.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn data_str(data: &Value, key: &str) -> Option<String> {
    data.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn data_i32(data: &Value, key: &str) -> Option<i32> {
    data.get(key).and_then(|v| v.as_i64()).map(|n| n as i32)
}

fn data_i64(data: &Value, key: &str) -> Option<i64> {
    data.get(key).and_then(|v| v.as_i64())
}

fn now_fixed() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap())
}

async fn save_error(
    db: &DatabaseConnection,
    project: &models::project::Model,
    data: &Value,
    ctx: &Value,
    alert_tx: &tokio::sync::broadcast::Sender<alert_service::AlertEvent>,
    cfg: &Config,
) -> AppResult<()> {
    let active = models::js_error::ActiveModel {
        id: sea_orm::NotSet,
        project_id: Set(project.id),
        app_id: Set(project.app_id.clone()),
        message: Set(data_str(data, "message").unwrap_or_default()),
        stack: Set(data_str(data, "stack")),
        error_type: Set(data_str(data, "type").unwrap_or_else(|| "js".into())),
        source_url: Set(data_str(data, "source_url")),
        line: Set(data_i32(data, "line")),
        column: Set(data_i32(data, "column")),
        user_agent: Set(ctx_str(ctx, "user_agent")),
        browser: Set(ctx_str(ctx, "browser")),
        browser_version: Set(ctx_str(ctx, "browser_version")),
        os: Set(ctx_str(ctx, "os")),
        os_version: Set(ctx_str(ctx, "os_version")),
        device: Set(ctx_str(ctx, "device")),
        device_type: Set(ctx_str(ctx, "device_type")),
        url: Set(ctx_str(ctx, "url")),
        referrer: Set(ctx_str(ctx, "referrer")),
        viewport: Set(ctx_str(ctx, "viewport")),
        screen_resolution: Set(ctx_str(ctx, "screen_resolution")),
        language: Set(ctx_str(ctx, "language")),
        timezone: Set(ctx_str(ctx, "timezone")),
        breadcrumb: Set(ctx.get("breadcrumb").cloned()),
        extra: Set(data.get("extra").cloned()),
        fingerprint: Set(data_str(data, "fingerprint")),
        sdk_version: Set(ctx_str(ctx, "sdk_version")),
        release: Set(ctx_str(ctx, "release")),
        environment: Set(ctx_str(ctx, "environment")),
        is_ai_analyzed: Set(false),
        distinct_id: Set(ctx_str(ctx, "distinct_id")),
        created_at: Set(now_fixed()),
    };
    let saved = active.insert(db).await?;
    alert_service::check_on_new_error(db, alert_tx, project.id, &saved).await;
    maybe_auto_analyze(db, cfg, &saved).await;
    Ok(())
}

async fn maybe_auto_analyze(
    db: &DatabaseConnection,
    cfg: &Config,
    error: &models::js_error::Model,
) {
    if !cfg.ai_enabled || cfg.ai_api_key.is_empty() {
        return;
    }
    let Some(fp) = error.fingerprint.as_deref() else {
        return;
    };

    let total_same_fp = models::JsError::find()
        .filter(models::js_error::Column::ProjectId.eq(error.project_id))
        .filter(models::js_error::Column::Fingerprint.eq(fp))
        .count(db)
        .await
        .unwrap_or(0);

    let since = (Utc::now() - Duration::hours(1)).with_timezone(&FixedOffset::east_opt(0).unwrap());
    let recent_same_fp = models::JsError::find()
        .filter(models::js_error::Column::ProjectId.eq(error.project_id))
        .filter(models::js_error::Column::Fingerprint.eq(fp))
        .filter(models::js_error::Column::CreatedAt.gte(since))
        .count(db)
        .await
        .unwrap_or(0);

    if total_same_fp > 1 && recent_same_fp <= 50 {
        return;
    }

    let db_clone = db.clone();
    let cfg_clone = cfg.clone();
    let error_clone = error.clone();
    tokio::spawn(async move {
        if let Err(e) = ai_service::analyze_error(&db_clone, &cfg_clone, &error_clone).await {
            tracing::warn!(error_id = error_clone.id, error = %e, "automatic AI analysis failed");
        }
    });
}

async fn save_network(
    db: &DatabaseConnection,
    project: &models::project::Model,
    data: &Value,
    ctx: &Value,
) -> AppResult<()> {
    let active = models::network_error::ActiveModel {
        id: sea_orm::NotSet,
        project_id: Set(project.id),
        app_id: Set(project.app_id.clone()),
        url: Set(data_str(data, "url").unwrap_or_default()),
        method: Set(data_str(data, "method").unwrap_or_else(|| "GET".into())),
        status: Set(data_i32(data, "status")),
        request_headers: Set(data.get("request_headers").cloned()),
        request_body: Set(data_str(data, "request_body")),
        response_headers: Set(data.get("response_headers").cloned()),
        response_text: Set(data_str(data, "response_text")),
        duration: Set(data_i32(data, "duration")),
        error_type: Set(data_str(data, "error_type")),
        user_agent: Set(ctx_str(ctx, "user_agent")),
        browser: Set(ctx_str(ctx, "browser")),
        os: Set(ctx_str(ctx, "os")),
        device: Set(ctx_str(ctx, "device_type")),
        sdk_version: Set(ctx_str(ctx, "sdk_version")),
        release: Set(ctx_str(ctx, "release")),
        environment: Set(ctx_str(ctx, "environment")),
        page_url: Set(ctx_str(ctx, "url")),
        distinct_id: Set(ctx_str(ctx, "distinct_id")),
        created_at: Set(now_fixed()),
    };
    active.insert(db).await?;
    Ok(())
}

async fn save_performance(
    db: &DatabaseConnection,
    project: &models::project::Model,
    data: &Value,
    ctx: &Value,
) -> AppResult<()> {
    use sea_orm::prelude::Decimal;
    use std::str::FromStr;

    let cls = data
        .get("cls")
        .and_then(|v| v.as_f64())
        .and_then(|f| Decimal::from_str(&format!("{f:.4}")).ok());

    let active = models::performance_datum::ActiveModel {
        id: sea_orm::NotSet,
        project_id: Set(project.id),
        app_id: Set(project.app_id.clone()),
        url: Set(data_str(data, "url").or_else(|| ctx_str(ctx, "url"))),
        fp: Set(data_i32(data, "fp")),
        fcp: Set(data_i32(data, "fcp")),
        lcp: Set(data_i32(data, "lcp")),
        cls: Set(cls),
        ttfb: Set(data_i32(data, "ttfb")),
        fid: Set(data_i32(data, "fid")),
        load_time: Set(data_i32(data, "load_time")),
        dns_time: Set(data_i32(data, "dns_time")),
        tcp_time: Set(data_i32(data, "tcp_time")),
        ssl_time: Set(data_i32(data, "ssl_time")),
        dom_parse_time: Set(data_i32(data, "dom_parse_time")),
        resource_count: Set(data_i32(data, "resource_count")),
        resource_size: Set(data_i64(data, "resource_size")),
        user_agent: Set(ctx_str(ctx, "user_agent")),
        browser: Set(ctx_str(ctx, "browser")),
        device_type: Set(ctx_str(ctx, "device_type")),
        sdk_version: Set(ctx_str(ctx, "sdk_version")),
        release: Set(ctx_str(ctx, "release")),
        environment: Set(ctx_str(ctx, "environment")),
        created_at: Set(now_fixed()),
    };
    active.insert(db).await?;
    Ok(())
}
