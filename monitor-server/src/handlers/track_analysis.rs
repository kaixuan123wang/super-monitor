//! 埋点事件分析查询接口（Phase 3）。
//!
//! GET /api/track/analysis   事件分析（时序折线图 + 分组维度）

use axum::{extract::{Query, State}, Json};
use chrono::{Duration, Utc};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashSet;

use crate::error::{AppError, AppResult};
use crate::models;
use crate::router::AppState;

#[derive(Debug, Deserialize)]
pub struct AnalysisQuery {
    pub project_id: i32,
    /// 事件名（支持多个，逗号分隔）
    pub events: String,
    /// 时间范围（天数），默认 7
    #[serde(default = "default_days")]
    pub days: i64,
    /// 指标类型：pv（total）/ uv（unique users）
    #[serde(default = "default_metric")]
    pub metric: String,
    /// 分组维度（可选）：browser / os / device_type / environment
    pub group_by: Option<String>,
}

fn default_days() -> i64 { 7 }
fn default_metric() -> String { "pv".into() }

pub async fn event_analysis(
    State(state): State<AppState>,
    Query(q): Query<AnalysisQuery>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;

    let event_names: Vec<&str> = q.events.split(',').map(str::trim).collect();
    let since = (Utc::now() - Duration::days(q.days))
        .with_timezone(&chrono::FixedOffset::east_opt(0).unwrap());

    let rows = models::TrackEvent::find()
        .filter(models::track_event::Column::ProjectId.eq(q.project_id))
        .filter(models::track_event::Column::CreatedAt.gte(since))
        .order_by_asc(models::track_event::Column::CreatedAt)
        .limit(10000)
        .all(db)
        .await?;

    let rows: Vec<_> = rows
        .into_iter()
        .filter(|r| event_names.contains(&r.event.as_str()))
        .collect();

    // 日期列表（升序）
    let dates: Vec<String> = (0..q.days)
        .rev()
        .map(|i| (Utc::now() - Duration::days(i)).date_naive().to_string())
        .collect();

    let series_data = if let Some(group_dim) = &q.group_by {
        build_grouped_series(&rows, &event_names, &dates, group_dim, &q.metric)
    } else {
        build_event_series(&rows, &event_names, &dates, &q.metric)
    };

    Ok(Json(json!({
        "code": 0, "message": "ok",
        "data": { "dates": dates, "series": series_data }
    })))
}

fn row_date(row: &models::track_event::Model) -> String {
    row.created_at.date_naive().to_string()
}

fn count_metric(rows: &[&models::track_event::Model], metric: &str) -> u64 {
    if metric == "uv" {
        let uniq: HashSet<&str> = rows.iter().map(|r| r.distinct_id.as_str()).collect();
        uniq.len() as u64
    } else {
        rows.len() as u64
    }
}

fn build_event_series(
    rows: &[models::track_event::Model],
    event_names: &[&str],
    dates: &[String],
    metric: &str,
) -> Vec<Value> {
    event_names
        .iter()
        .map(|&ev| {
            let data: Vec<u64> = dates
                .iter()
                .map(|d| {
                    let day: Vec<&models::track_event::Model> = rows
                        .iter()
                        .filter(|r| r.event == ev && row_date(r) == *d)
                        .collect();
                    count_metric(&day, metric)
                })
                .collect();
            json!({ "name": ev, "data": data })
        })
        .collect()
}

fn build_grouped_series(
    rows: &[models::track_event::Model],
    event_names: &[&str],
    dates: &[String],
    group_dim: &str,
    metric: &str,
) -> Vec<Value> {
    let mut group_values: Vec<String> = rows
        .iter()
        .filter(|r| event_names.contains(&r.event.as_str()))
        .map(|r| dim_value(r, group_dim))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    group_values.sort();

    group_values
        .iter()
        .map(|gv| {
            let data: Vec<u64> = dates
                .iter()
                .map(|d| {
                    let day: Vec<&models::track_event::Model> = rows
                        .iter()
                        .filter(|r| {
                            event_names.contains(&r.event.as_str())
                                && row_date(r) == *d
                                && dim_value(r, group_dim) == *gv
                        })
                        .collect();
                    count_metric(&day, metric)
                })
                .collect();
            json!({ "name": gv, "data": data })
        })
        .collect()
}

fn dim_value(row: &models::track_event::Model, dim: &str) -> String {
    match dim {
        "browser" => row.browser.clone().unwrap_or_else(|| "Unknown".into()),
        "os" => row.os.clone().unwrap_or_else(|| "Unknown".into()),
        "device_type" => row.device_type.clone().unwrap_or_else(|| "desktop".into()),
        "environment" => row.environment.clone().unwrap_or_else(|| "production".into()),
        _ => "Unknown".into(),
    }
}

fn get_db(state: &crate::router::AppState) -> AppResult<&sea_orm::DatabaseConnection> {
    state
        .db
        .as_ref()
        .ok_or_else(|| AppError::Internal("database not connected".into()))
}
