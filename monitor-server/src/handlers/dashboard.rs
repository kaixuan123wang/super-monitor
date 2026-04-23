//! 仪表盘统计数据 + SSE 实时推送。
//!
//! GET /api/dashboard/overview   概览统计（错误数/趋势/分布/性能均值/Top 错误）
//! GET /api/dashboard/realtime   SSE 长连接，推送 error / heartbeat 事件

use axum::response::sse::{Event, KeepAlive};
use axum::{
    extract::{Query, State},
    response::Sse,
    Json,
};
use chrono::{Duration, Utc};
use futures_util::stream;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::convert::Infallible;
use std::time::Duration as StdDuration;

use crate::error::{AppError, AppResult};
use crate::models;
use crate::router::AppState;

// ── Query params ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct OverviewQuery {
    pub project_id: i32,
    #[serde(default = "default_days")]
    pub days: i64,
}

fn default_days() -> i64 {
    7
}

// ── overview 接口 ──────────────────────────────────────────────────────────────

pub async fn overview(
    State(state): State<AppState>,
    Query(q): Query<OverviewQuery>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let since = Utc::now() - Duration::days(q.days);
    let since_fixed = since.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap());

    // 总错误数
    let total_errors = models::JsError::find()
        .filter(models::js_error::Column::ProjectId.eq(q.project_id))
        .filter(models::js_error::Column::CreatedAt.gte(since_fixed))
        .count(db)
        .await?;

    // 总网络错误数
    let total_network_errors = models::NetworkError::find()
        .filter(models::network_error::Column::ProjectId.eq(q.project_id))
        .filter(models::network_error::Column::CreatedAt.gte(since_fixed))
        .count(db)
        .await?;

    // 错误趋势（最近 days 天，每天一个数据点）
    let error_trend = build_error_trend(db, q.project_id, q.days).await?;

    // 浏览器分布
    let browser_distribution = build_browser_distribution(db, q.project_id, since_fixed).await?;

    // OS 分布
    let os_distribution = build_os_distribution(db, q.project_id, since_fixed).await?;

    // 设备类型分布
    let device_distribution = build_device_distribution(db, q.project_id, since_fixed).await?;

    // Top 错误（按出现次数）
    let top_errors = build_top_errors(db, q.project_id, since_fixed).await?;

    // 平均性能指标
    let avg_performance = build_avg_performance(db, q.project_id, since_fixed).await?;

    Ok(Json(json!({
        "code": 0,
        "message": "ok",
        "data": {
            "total_errors": total_errors,
            "total_network_errors": total_network_errors,
            "error_trend": error_trend,
            "browser_distribution": browser_distribution,
            "os_distribution": os_distribution,
            "device_distribution": device_distribution,
            "top_errors": top_errors,
            "avg_performance": avg_performance,
        }
    })))
}

// ── 统计辅助函数 ───────────────────────────────────────────────────────────────

/// 按天聚合错误数（返回 [{date, count}]）。
async fn build_error_trend(
    db: &DatabaseConnection,
    project_id: i32,
    days: i64,
) -> AppResult<Vec<Value>> {
    let mut trend = Vec::new();
    for i in (0..days).rev() {
        let day_start = (Utc::now() - Duration::days(i)).date_naive();
        let day_end = day_start + chrono::Days::new(1);
        let s = day_start
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .with_timezone(&chrono::FixedOffset::east_opt(0).unwrap());
        let e = day_end
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .with_timezone(&chrono::FixedOffset::east_opt(0).unwrap());

        let count = models::JsError::find()
            .filter(models::js_error::Column::ProjectId.eq(project_id))
            .filter(models::js_error::Column::CreatedAt.gte(s))
            .filter(models::js_error::Column::CreatedAt.lt(e))
            .count(db)
            .await?;

        trend.push(json!({ "date": day_start.to_string(), "count": count }));
    }
    Ok(trend)
}

/// 浏览器分布（取前 10）。
async fn build_browser_distribution(
    db: &DatabaseConnection,
    project_id: i32,
    since: chrono::DateTime<chrono::FixedOffset>,
) -> AppResult<Vec<Value>> {
    let rows = models::JsError::find()
        .filter(models::js_error::Column::ProjectId.eq(project_id))
        .filter(models::js_error::Column::CreatedAt.gte(since))
        .all(db)
        .await?;

    let mut map: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for row in &rows {
        let key = row.browser.clone().unwrap_or_else(|| "Unknown".into());
        *map.entry(key).or_default() += 1;
    }
    let mut result: Vec<(String, u64)> = map.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1));
    result.truncate(10);
    Ok(result
        .into_iter()
        .map(|(name, value)| json!({ "name": name, "value": value }))
        .collect())
}

/// OS 分布（取前 10）。
async fn build_os_distribution(
    db: &DatabaseConnection,
    project_id: i32,
    since: chrono::DateTime<chrono::FixedOffset>,
) -> AppResult<Vec<Value>> {
    let rows = models::JsError::find()
        .filter(models::js_error::Column::ProjectId.eq(project_id))
        .filter(models::js_error::Column::CreatedAt.gte(since))
        .all(db)
        .await?;

    let mut map: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for row in &rows {
        let key = row.os.clone().unwrap_or_else(|| "Unknown".into());
        *map.entry(key).or_default() += 1;
    }
    let mut result: Vec<(String, u64)> = map.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1));
    result.truncate(10);
    Ok(result
        .into_iter()
        .map(|(name, value)| json!({ "name": name, "value": value }))
        .collect())
}

/// 设备类型分布。
async fn build_device_distribution(
    db: &DatabaseConnection,
    project_id: i32,
    since: chrono::DateTime<chrono::FixedOffset>,
) -> AppResult<Vec<Value>> {
    let rows = models::JsError::find()
        .filter(models::js_error::Column::ProjectId.eq(project_id))
        .filter(models::js_error::Column::CreatedAt.gte(since))
        .all(db)
        .await?;

    let mut map: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for row in &rows {
        let key = row.device_type.clone().unwrap_or_else(|| "desktop".into());
        *map.entry(key).or_default() += 1;
    }
    let mut result: Vec<(String, u64)> = map.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1));
    Ok(result
        .into_iter()
        .map(|(name, value)| json!({ "name": name, "value": value }))
        .collect())
}

/// Top 错误（按 fingerprint 聚合，取前 10）。
async fn build_top_errors(
    db: &DatabaseConnection,
    project_id: i32,
    since: chrono::DateTime<chrono::FixedOffset>,
) -> AppResult<Vec<Value>> {
    let rows = models::JsError::find()
        .filter(models::js_error::Column::ProjectId.eq(project_id))
        .filter(models::js_error::Column::CreatedAt.gte(since))
        .order_by_desc(models::js_error::Column::CreatedAt)
        .limit(1000)
        .all(db)
        .await?;

    let mut map: std::collections::HashMap<String, (String, u64)> =
        std::collections::HashMap::new();
    for row in &rows {
        let key = row
            .fingerprint
            .clone()
            .unwrap_or_else(|| row.message.chars().take(80).collect());
        let entry = map
            .entry(key.clone())
            .or_insert_with(|| (row.message.chars().take(120).collect(), 0));
        entry.1 += 1;
    }
    let mut result: Vec<(String, String, u64)> = map
        .into_iter()
        .map(|(fp, (msg, cnt))| (fp, msg, cnt))
        .collect();
    result.sort_by(|a, b| b.2.cmp(&a.2));
    result.truncate(10);
    Ok(result
        .into_iter()
        .map(
            |(fp, message, count)| json!({ "fingerprint": fp, "message": message, "count": count }),
        )
        .collect())
}

/// 平均性能指标（fp/fcp/lcp/cls/ttfb）。
async fn build_avg_performance(
    db: &DatabaseConnection,
    project_id: i32,
    since: chrono::DateTime<chrono::FixedOffset>,
) -> AppResult<Value> {
    let rows = models::PerformanceDatum::find()
        .filter(models::performance_datum::Column::ProjectId.eq(project_id))
        .filter(models::performance_datum::Column::CreatedAt.gte(since))
        .limit(2000)
        .all(db)
        .await?;

    if rows.is_empty() {
        return Ok(json!({ "fp": null, "fcp": null, "lcp": null, "cls": null, "ttfb": null }));
    }

    let avg = |f: &dyn Fn(&models::performance_datum::Model) -> Option<i32>| -> Option<f64> {
        let vals: Vec<f64> = rows.iter().filter_map(|r| f(r).map(|v| v as f64)).collect();
        if vals.is_empty() {
            None
        } else {
            Some(vals.iter().sum::<f64>() / vals.len() as f64)
        }
    };

    Ok(json!({
        "fp":   avg(&|r| r.fp),
        "fcp":  avg(&|r| r.fcp),
        "lcp":  avg(&|r| r.lcp),
        "ttfb": avg(&|r| r.ttfb),
    }))
}

// ── SSE 实时推送 ───────────────────────────────────────────────────────────────

struct SseState {
    db: Option<sea_orm::DatabaseConnection>,
    project_id: i32,
    last_error_id: i64,
    tick: u64,
    alert_rx: tokio::sync::broadcast::Receiver<crate::services::alert_service::AlertEvent>,
}

pub async fn realtime(
    State(state): State<AppState>,
    Query(q): Query<OverviewQuery>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    let alert_rx = state.alert_tx.subscribe();
    let init_state = SseState {
        db: state.db.clone(),
        project_id: q.project_id,
        last_error_id: 0,
        tick: 0,
        alert_rx,
    };

    let stream = stream::unfold(init_state, move |mut s| async move {
        match s.tick {
            0 => {
                let cursor = if let Some(db) = &s.db {
                    fetch_latest_error_id(db, s.project_id).await.unwrap_or(0)
                } else {
                    0
                };
                s.last_error_id = cursor;
                s.tick = 1;

                let event = Event::default().event("init").data(
                    json!({
                        "project_id": s.project_id,
                        "connection_id": uuid::Uuid::new_v4(),
                    })
                    .to_string(),
                );
                Some((Ok(event), s))
            }
            tick if tick % 6 == 0 => {
                tokio::time::sleep(StdDuration::from_secs(5)).await;
                s.tick += 1;
                let event = Event::default()
                    .event("heartbeat")
                    .data(json!({ "timestamp": Utc::now().timestamp_millis() }).to_string());
                Some((Ok(event), s))
            }
            _ => {
                // 先检查 alert 广播（非阻塞）
                while let Ok(alert) = s.alert_rx.try_recv() {
                    if alert.project_id == s.project_id {
                        s.tick += 1;
                        let event = Event::default()
                            .event("alert")
                            .data(serde_json::to_string(&alert).unwrap_or_default());
                        return Some((Ok(event), s));
                    }
                }

                tokio::time::sleep(StdDuration::from_secs(5)).await;
                s.tick += 1;

                if let Some(db) = &s.db {
                    match fetch_new_errors(db, s.project_id, s.last_error_id).await {
                        Ok(errors) if !errors.is_empty() => {
                            if let Some(max_id) = errors.iter().map(|e| e.id).max() {
                                s.last_error_id = max_id;
                            }
                            let e = &errors[0];
                            let event = Event::default().event("error").data(
                                json!({
                                    "id": e.id,
                                    "message": e.message,
                                    "error_type": e.error_type,
                                    "created_at": e.created_at,
                                    "project_id": e.project_id,
                                })
                                .to_string(),
                            );
                            if errors.len() > 1 {
                                s.last_error_id = errors[0].id;
                            }
                            return Some((Ok(event), s));
                        }
                        _ => {}
                    }
                }

                let event = Event::default().comment("poll");
                Some((Ok(event), s))
            }
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn fetch_latest_error_id(
    db: &sea_orm::DatabaseConnection,
    project_id: i32,
) -> AppResult<i64> {
    let row = models::JsError::find()
        .filter(models::js_error::Column::ProjectId.eq(project_id))
        .order_by_desc(models::js_error::Column::Id)
        .one(db)
        .await?;
    Ok(row.map(|r| r.id).unwrap_or(0))
}

async fn fetch_new_errors(
    db: &sea_orm::DatabaseConnection,
    project_id: i32,
    after_id: i64,
) -> AppResult<Vec<models::js_error::Model>> {
    let rows = models::JsError::find()
        .filter(models::js_error::Column::ProjectId.eq(project_id))
        .filter(models::js_error::Column::Id.gt(after_id))
        .order_by_asc(models::js_error::Column::Id)
        .limit(10)
        .all(db)
        .await?;
    Ok(rows)
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn get_db(state: &AppState) -> AppResult<&DatabaseConnection> {
    state
        .db
        .as_ref()
        .ok_or_else(|| AppError::Internal("database not connected".into()))
}
