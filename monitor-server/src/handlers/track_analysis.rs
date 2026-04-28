//! 埋点事件分析查询接口（Phase 3 + Phase 4）。

use axum::response::sse::{Event, KeepAlive};
use axum::{
    extract::{Extension, Path, Query, State},
    response::Sse,
    Json,
};
use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate, Utc};
use futures_util::stream;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;

use crate::error::{AppError, AppResult};
use crate::middleware::auth::{check_project_access, CurrentUser};
use crate::models;
use crate::router::AppState;
use crate::utils::validate_sse_token;

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

fn default_days() -> i64 {
    7
}
fn default_metric() -> String {
    "pv".into()
}

pub async fn event_analysis(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Query(q): Query<AnalysisQuery>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    check_project_access(db, &current_user, q.project_id).await?;

    let event_names: Vec<&str> = q.events.split(',').map(str::trim).collect();
    let days = q.days.clamp(1, 90);
    let since = (Utc::now() - Duration::days(days))
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
    let dates: Vec<String> = (0..days)
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
        "environment" => row
            .environment
            .clone()
            .unwrap_or_else(|| "production".into()),
        _ => "Unknown".into(),
    }
}

fn get_db(state: &crate::router::AppState) -> AppResult<&sea_orm::DatabaseConnection> {
    state
        .db
        .as_ref()
        .ok_or_else(|| AppError::Internal("database not connected".into()))
}

fn now_fixed() -> chrono::DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap())
}

fn parse_time_bound(value: Option<&str>, end_of_day: bool) -> Option<DateTime<FixedOffset>> {
    let raw = value?.trim();
    if raw.is_empty() {
        return None;
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
        return Some(dt);
    }
    let date = NaiveDate::parse_from_str(raw, "%Y-%m-%d").ok()?;
    let time = if end_of_day {
        date.and_hms_opt(23, 59, 59)?
    } else {
        date.and_hms_opt(0, 0, 0)?
    };
    Some(DateTime::<FixedOffset>::from_naive_utc_and_offset(
        time,
        FixedOffset::east_opt(0).unwrap(),
    ))
}

fn json_value_matches(actual: &Value, expected: &Value) -> bool {
    match expected {
        Value::Array(items) => items.iter().any(|item| json_value_matches(actual, item)),
        Value::String(expected_str) => actual
            .as_str()
            .map(|actual_str| actual_str == expected_str)
            .unwrap_or(false),
        Value::Number(expected_num) => {
            actual == expected || actual.as_f64() == expected_num.as_f64()
        }
        Value::Bool(expected_bool) => actual.as_bool() == Some(*expected_bool),
        Value::Null => actual.is_null(),
        Value::Object(_) => actual == expected,
    }
}

fn json_lookup(source: &Option<Value>, key: &str) -> Option<Value> {
    source
        .as_ref()
        .and_then(|v| v.as_object())
        .and_then(|m| m.get(key))
        .cloned()
}

fn event_field_value(row: &models::track_event::Model, key: &str) -> Option<Value> {
    let str_value = match key {
        "event" => Some(row.event.clone()),
        "event_type" => Some(row.event_type.clone()),
        "distinct_id" => Some(row.distinct_id.clone()),
        "anonymous_id" => row.anonymous_id.clone(),
        "user_id" => row.user_id.clone(),
        "session_id" => row.session_id.clone(),
        "page_url" => row.page_url.clone(),
        "page_title" => row.page_title.clone(),
        "browser" => row.browser.clone(),
        "os" => row.os.clone(),
        "device_type" => row.device_type.clone(),
        "environment" => row.environment.clone(),
        "release" => row.release.clone(),
        "language" => row.language.clone(),
        _ => None,
    };
    str_value
        .map(Value::String)
        .or_else(|| json_lookup(&row.properties, key))
        .or_else(|| json_lookup(&row.super_properties, key))
}

fn event_matches_filters(row: &models::track_event::Model, filters: Option<&Value>) -> bool {
    let Some(filter_obj) = filters.and_then(|v| v.as_object()) else {
        return true;
    };

    filter_obj.iter().all(|(key, expected)| {
        event_field_value(row, key)
            .map(|actual| json_value_matches(&actual, expected))
            .unwrap_or(false)
    })
}

fn event_matches_step(row: &models::track_event::Model, step: &Value) -> bool {
    let event_name = step.get("event").and_then(|v| v.as_str()).unwrap_or("");
    row.event == event_name && event_matches_filters(row, step.get("filters"))
}

fn dim_or_property_value(row: &models::track_event::Model, dim: &str) -> String {
    event_field_value(row, dim)
        .and_then(|v| {
            v.as_str().map(|s| s.to_string()).or_else(|| {
                if v.is_null() {
                    None
                } else {
                    Some(v.to_string())
                }
            })
        })
        .unwrap_or_else(|| "Unknown".into())
}

// ═══════════════════════════════════════════════════════════════════
// Phase 4 — 漏斗分析
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct FunnelListQuery {
    pub project_id: i32,
}

pub async fn list_funnels(
    State(state): State<crate::router::AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Query(q): Query<FunnelListQuery>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    check_project_access(db, &current_user, q.project_id).await?;
    let items = models::TrackFunnel::find()
        .filter(models::track_funnel::Column::ProjectId.eq(q.project_id))
        .order_by_desc(models::track_funnel::Column::CreatedAt)
        .all(db)
        .await?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": { "list": items, "total": items.len() } })))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateFunnelBody {
    pub project_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub steps: Value,
    #[serde(default = "default_window")]
    pub window_minutes: i32,
}
fn default_window() -> i32 {
    1440
}

pub async fn create_funnel(
    State(state): State<crate::router::AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(body): Json<CreateFunnelBody>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    check_project_access(db, &current_user, body.project_id).await?;
    let am = models::track_funnel::ActiveModel {
        id: sea_orm::NotSet,
        project_id: Set(body.project_id),
        name: Set(body.name),
        description: Set(body.description),
        steps: Set(body.steps),
        window_minutes: Set(body.window_minutes),
        created_by: Set(Some(current_user.id)),
        created_at: Set(now_fixed()),
        updated_at: Set(now_fixed()),
    };
    let saved = am.insert(db).await?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": saved })))
}

pub async fn get_funnel(
    State(state): State<crate::router::AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i32>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let row = models::TrackFunnel::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    check_project_access(db, &current_user, row.project_id).await?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": row })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateFunnelBody {
    pub name: Option<String>,
    pub description: Option<String>,
    pub steps: Option<Value>,
    pub window_minutes: Option<i32>,
}

pub async fn update_funnel(
    State(state): State<crate::router::AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i32>,
    Json(body): Json<UpdateFunnelBody>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let row = models::TrackFunnel::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    check_project_access(db, &current_user, row.project_id).await?;
    let mut am: models::track_funnel::ActiveModel = row.into();
    if let Some(v) = body.name {
        am.name = Set(v);
    }
    if let Some(v) = body.description {
        am.description = Set(Some(v));
    }
    if let Some(v) = body.steps {
        am.steps = Set(v);
    }
    if let Some(v) = body.window_minutes {
        am.window_minutes = Set(v);
    }
    am.updated_at = Set(now_fixed());
    let updated = am.update(db).await?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": updated })))
}

pub async fn delete_funnel(
    State(state): State<crate::router::AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i32>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let row = models::TrackFunnel::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    check_project_access(db, &current_user, row.project_id).await?;
    models::TrackFunnel::delete_by_id(id).exec(db).await?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": { "deleted": 1 } })))
}

#[derive(Debug, Deserialize)]
pub struct FunnelAnalyzeBody {
    pub time_range: Option<FunnelTimeRange>,
    pub group_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FunnelTimeRange {
    pub start: Option<String>,
    pub end: Option<String>,
    pub days: Option<i64>,
}

fn funnel_time_bounds(
    range: Option<&FunnelTimeRange>,
) -> (DateTime<FixedOffset>, DateTime<FixedOffset>) {
    let now = now_fixed();
    let days = range.and_then(|t| t.days).unwrap_or(7).max(1);
    let end = parse_time_bound(range.and_then(|t| t.end.as_deref()), true).unwrap_or(now);
    let start = parse_time_bound(range.and_then(|t| t.start.as_deref()), false)
        .unwrap_or_else(|| end - Duration::days(days));
    (start, end)
}

pub async fn analyze_funnel(
    State(state): State<crate::router::AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i32>,
    Json(body): Json<FunnelAnalyzeBody>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let funnel = models::TrackFunnel::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    check_project_access(db, &current_user, funnel.project_id).await?;

    let (since, until) = funnel_time_bounds(body.time_range.as_ref());

    let steps: Vec<Value> = serde_json::from_value(funnel.steps.clone()).unwrap_or_default();

    if steps.is_empty() {
        return Ok(Json(
            json!({ "code": 0, "message": "ok", "data": { "steps": [], "overall_conversion": 0 } }),
        ));
    }

    let window = Duration::minutes(funnel.window_minutes as i64);

    // 读取所有相关事件
    let all_events = models::TrackEvent::find()
        .filter(models::track_event::Column::ProjectId.eq(funnel.project_id))
        .filter(models::track_event::Column::CreatedAt.gte(since))
        .filter(models::track_event::Column::CreatedAt.lte(until))
        .order_by_asc(models::track_event::Column::CreatedAt)
        .limit(50000)
        .all(db)
        .await?;

    // 按 distinct_id 分组
    let mut user_events: HashMap<String, Vec<&models::track_event::Model>> = HashMap::new();
    for e in &all_events {
        user_events
            .entry(e.distinct_id.clone())
            .or_default()
            .push(e);
    }

    let (step_results, overall) = compute_funnel_steps(&user_events, &steps, window);

    let breakdown = body.group_by.as_deref().map(|group_dim| {
        let first_step = &steps[0];
        let mut grouped_users: HashMap<String, HashMap<String, Vec<&models::track_event::Model>>> =
            HashMap::new();
        for (uid, events) in &user_events {
            if let Some(first_event) = events.iter().find(|e| event_matches_step(e, first_step)) {
                let group_value = dim_or_property_value(first_event, group_dim);
                grouped_users
                    .entry(group_value)
                    .or_default()
                    .insert(uid.clone(), events.clone());
            }
        }

        let mut items: Vec<Value> = grouped_users
            .into_iter()
            .map(|(group, rows)| {
                let (steps, overall_conversion) = compute_funnel_steps(&rows, &steps, window);
                json!({
                    "group": group,
                    "steps": steps,
                    "overall_conversion": (overall_conversion * 10000.0).round() / 10000.0,
                })
            })
            .collect();
        items.sort_by(|a, b| a["group"].as_str().cmp(&b["group"].as_str()));
        items
    });

    Ok(Json(json!({
        "code": 0, "message": "ok",
        "data": {
            "steps": step_results,
            "overall_conversion": (overall * 10000.0).round() / 10000.0,
            "breakdown": breakdown.unwrap_or_default(),
        }
    })))
}

fn compute_funnel_steps(
    user_events: &HashMap<String, Vec<&models::track_event::Model>>,
    steps: &[Value],
    window: Duration,
) -> (Vec<Value>, f64) {
    let mut step_results: Vec<Value> = Vec::new();
    let mut prev_conversions: HashMap<String, DateTime<FixedOffset>> = HashMap::new();
    let mut first_count = 0u64;

    for (idx, step) in steps.iter().enumerate() {
        let event_name = step.get("event").and_then(|v| v.as_str()).unwrap_or("");
        let mut conversions: HashMap<String, DateTime<FixedOffset>> = HashMap::new();
        let mut total_time_ms: i64 = 0;
        let mut time_count = 0i64;

        if idx == 0 {
            for (uid, events) in user_events {
                if let Some(first_hit) = events.iter().find(|e| event_matches_step(e, step)) {
                    conversions.insert(uid.clone(), first_hit.created_at);
                }
            }
        } else {
            for (uid, prev_time) in &prev_conversions {
                let Some(events) = user_events.get(uid) else {
                    continue;
                };
                let deadline = *prev_time + window;
                if let Some(next_hit) = events.iter().find(|e| {
                    event_matches_step(e, step)
                        && e.created_at > *prev_time
                        && e.created_at <= deadline
                }) {
                    conversions.insert(uid.clone(), next_hit.created_at);
                    let diff = (next_hit.created_at - *prev_time).num_milliseconds();
                    if diff > 0 {
                        total_time_ms += diff;
                        time_count += 1;
                    }
                }
            }
        }

        let user_count = conversions.len() as u64;
        if idx == 0 {
            first_count = user_count;
        }
        let previous_count = if idx == 0 {
            user_count
        } else {
            prev_conversions.len() as u64
        };
        let conversion_rate = if first_count > 0 {
            user_count as f64 / first_count as f64
        } else {
            0.0
        };
        let step_conversion_rate = if previous_count > 0 {
            user_count as f64 / previous_count as f64
        } else {
            0.0
        };
        let avg_time_to_next = if time_count > 0 {
            Some(total_time_ms / time_count)
        } else {
            None
        };

        step_results.push(json!({
            "event": event_name,
            "display_name": step.get("display_name").and_then(|v| v.as_str()).unwrap_or(event_name),
            "user_count": user_count,
            "conversion_rate": (conversion_rate * 10000.0).round() / 10000.0,
            "step_conversion_rate": (step_conversion_rate * 10000.0).round() / 10000.0,
            "avg_time_to_next_ms": avg_time_to_next,
        }));
        prev_conversions = conversions;
    }

    let last_count = step_results
        .last()
        .and_then(|s| s["user_count"].as_u64())
        .unwrap_or(0);
    let overall = if first_count > 0 {
        last_count as f64 / first_count as f64
    } else {
        0.0
    };
    (step_results, overall)
}

// ═══════════════════════════════════════════════════════════════════
// Phase 4 — 留存分析
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct RetentionListQuery {
    pub project_id: i32,
}

pub async fn list_retentions(
    State(state): State<crate::router::AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Query(q): Query<RetentionListQuery>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    check_project_access(db, &current_user, q.project_id).await?;
    let items = models::TrackRetentionConfig::find()
        .filter(models::track_retention_config::Column::ProjectId.eq(q.project_id))
        .order_by_desc(models::track_retention_config::Column::CreatedAt)
        .all(db)
        .await?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": { "list": items, "total": items.len() } })))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateRetentionBody {
    pub project_id: i32,
    pub name: String,
    pub initial_event: String,
    pub return_event: String,
    pub initial_filters: Option<Value>,
    pub return_filters: Option<Value>,
    #[serde(default = "default_retention_days")]
    pub retention_days: i32,
}
fn default_retention_days() -> i32 {
    7
}

pub async fn create_retention(
    State(state): State<crate::router::AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(body): Json<CreateRetentionBody>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    check_project_access(db, &current_user, body.project_id).await?;
    let am = models::track_retention_config::ActiveModel {
        id: sea_orm::NotSet,
        project_id: Set(body.project_id),
        name: Set(body.name),
        initial_event: Set(body.initial_event),
        return_event: Set(body.return_event),
        initial_filters: Set(body.initial_filters),
        return_filters: Set(body.return_filters),
        retention_days: Set(body.retention_days),
        created_by: Set(Some(current_user.id)),
        created_at: Set(now_fixed()),
    };
    let saved = am.insert(db).await?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": saved })))
}

#[derive(Debug, Deserialize)]
pub struct RetentionAnalyzeBody {
    pub time_range: Option<RetentionTimeRange>,
    pub retention_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RetentionTimeRange {
    pub days: Option<i64>,
    pub start: Option<String>,
    pub end: Option<String>,
}

fn retention_time_bounds(
    range: Option<&RetentionTimeRange>,
    _retention_days: i64,
) -> (DateTime<FixedOffset>, DateTime<FixedOffset>, i64) {
    let now = now_fixed();
    let cohort_days = range.and_then(|t| t.days).unwrap_or(14).max(1);
    let end = parse_time_bound(range.and_then(|t| t.end.as_deref()), true).unwrap_or(now);
    let start = parse_time_bound(range.and_then(|t| t.start.as_deref()), false)
        .unwrap_or_else(|| end - Duration::days(cohort_days));
    (start, end, cohort_days)
}

fn retention_bucket(date: NaiveDate, retention_type: &str) -> NaiveDate {
    if retention_type == "week" {
        date - Duration::days(date.weekday().num_days_from_monday() as i64)
    } else {
        date
    }
}

pub async fn analyze_retention(
    State(state): State<crate::router::AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i32>,
    Json(body): Json<RetentionAnalyzeBody>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let config = models::TrackRetentionConfig::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    check_project_access(db, &current_user, config.project_id).await?;

    let retention_type = body.retention_type.as_deref().unwrap_or("day");
    let retention_days = config.retention_days as i64;
    let (since, until, _cohort_days) =
        retention_time_bounds(body.time_range.as_ref(), retention_days);

    // 读取初始事件和回访事件
    let initial_events: Vec<_> = models::TrackEvent::find()
        .filter(models::track_event::Column::ProjectId.eq(config.project_id))
        .filter(models::track_event::Column::Event.eq(&config.initial_event))
        .filter(models::track_event::Column::CreatedAt.gte(since))
        .filter(models::track_event::Column::CreatedAt.lte(until))
        .order_by_asc(models::track_event::Column::CreatedAt)
        .limit(50000)
        .all(db)
        .await?
        .into_iter()
        .filter(|e| event_matches_filters(e, config.initial_filters.as_ref()))
        .collect();

    let return_events: Vec<_> = models::TrackEvent::find()
        .filter(models::track_event::Column::ProjectId.eq(config.project_id))
        .filter(models::track_event::Column::Event.eq(&config.return_event))
        .filter(models::track_event::Column::CreatedAt.gte(since))
        .filter(
            models::track_event::Column::CreatedAt
                .lte(until + Duration::days(retention_days.max(1))),
        )
        .limit(50000)
        .all(db)
        .await?
        .into_iter()
        .filter(|e| event_matches_filters(e, config.return_filters.as_ref()))
        .collect();

    // 按用户构建回访时间集合: distinct_id -> [bucket dates]
    let mut return_dates: HashMap<String, HashSet<NaiveDate>> = HashMap::new();
    for e in &return_events {
        return_dates
            .entry(e.distinct_id.clone())
            .or_default()
            .insert(retention_bucket(e.created_at.date_naive(), retention_type));
    }

    // 按天构建 cohort
    let mut cohort_table: Vec<Value> = Vec::new();
    let mut daily_avg: HashMap<i64, (f64, u32)> = HashMap::new();

    let start_date = since.date_naive();
    let end_date = until.date_naive();
    let total_days = (end_date - start_date).num_days().max(0) + 1;
    for day_offset in 0..total_days {
        let cohort_date = start_date + chrono::Days::new(day_offset as u64);
        if cohort_date > end_date {
            break;
        }
        if retention_type == "week" && cohort_date.weekday().num_days_from_monday() != 0 {
            continue;
        }
        let cohort_date_str = cohort_date.to_string();

        // cohort 成员：当天触发初始事件的用户
        let cohort_users: HashSet<String> = initial_events
            .iter()
            .filter(|e| retention_bucket(e.created_at.date_naive(), retention_type) == cohort_date)
            .map(|e| e.distinct_id.clone())
            .collect();

        if cohort_users.is_empty() {
            continue;
        }
        let cohort_size = cohort_users.len();
        let mut row = json!({
            "cohort_date": cohort_date_str,
            "cohort_size": cohort_size,
        });

        for d in 1..=retention_days {
            let target_date = if retention_type == "week" {
                cohort_date + chrono::Days::new((d * 7) as u64)
            } else {
                cohort_date + chrono::Days::new(d as u64)
            };
            let retained: usize = cohort_users
                .iter()
                .filter(|uid| {
                    return_dates
                        .get(*uid)
                        .is_some_and(|dates| dates.contains(&target_date))
                })
                .count();
            let rate = retained as f64 / cohort_size as f64;
            let key = if retention_type == "week" {
                format!("week_{}", d)
            } else {
                format!("day_{}", d)
            };
            row[key] = json!((rate * 10000.0).round() / 10000.0);

            let entry = daily_avg.entry(d).or_insert((0.0, 0));
            entry.0 += rate;
            entry.1 += 1;
        }
        cohort_table.push(row);
    }

    let mut avg_retention: Vec<f64> = Vec::new();
    for d in 1..=retention_days {
        let entry = daily_avg.get(&d).copied().unwrap_or((0.0, 0));
        let avg = if entry.1 > 0 {
            entry.0 / entry.1 as f64
        } else {
            0.0
        };
        avg_retention.push((avg * 10000.0).round() / 10000.0);
    }

    Ok(Json(json!({
        "code": 0, "message": "ok",
        "data": {
            "retention_table": cohort_table,
            "avg_retention": avg_retention,
            "retention_type": retention_type,
        }
    })))
}

// ═══════════════════════════════════════════════════════════════════
// Phase 4 — 实时事件流 SSE
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct LiveEventsQuery {
    pub project_id: i32,
    pub distinct_id: Option<String>,
    pub token: Option<String>,
}

#[derive(Clone)]
struct LiveState {
    db: Option<sea_orm::DatabaseConnection>,
    project_id: i32,
    distinct_id: Option<String>,
    last_id: i64,
    last_profile_updated_at: Option<DateTime<FixedOffset>>,
    tick: u64,
}

pub async fn live_events(
    State(state): State<crate::router::AppState>,
    Query(q): Query<LiveEventsQuery>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, AppError> {
    let current_user =
        validate_sse_token(&state, q.token.as_deref().ok_or(AppError::Unauthorized)?).await?;
    let db = get_db(&state)?;
    check_project_access(db, &current_user, q.project_id).await?;

    let init_state = LiveState {
        db: state.db.clone(),
        project_id: q.project_id,
        distinct_id: q.distinct_id,
        last_id: 0,
        last_profile_updated_at: None,
        tick: 0,
    };

    let stream = stream::unfold(init_state, |mut s| async move {
        match s.tick {
            0 => {
                // 初始化：获取当前最大 id 作为 cursor
                let cursor = if let Some(db) = &s.db {
                    let event_cursor = models::TrackEvent::find()
                        .filter(models::track_event::Column::ProjectId.eq(s.project_id))
                        .order_by_desc(models::track_event::Column::Id)
                        .one(db)
                        .await
                        .ok()
                        .flatten()
                        .map(|r| r.id)
                        .unwrap_or(0);
                    let mut profile_cursor_query = models::TrackUserProfile::find()
                        .filter(models::track_user_profile::Column::ProjectId.eq(s.project_id))
                        .order_by_desc(models::track_user_profile::Column::UpdatedAt)
                        .limit(1);
                    if let Some(did) = &s.distinct_id {
                        profile_cursor_query = profile_cursor_query.filter(
                            models::track_user_profile::Column::DistinctId.eq(did.as_str()),
                        );
                    }
                    s.last_profile_updated_at = profile_cursor_query
                        .one(db)
                        .await
                        .ok()
                        .flatten()
                        .map(|r| r.updated_at);
                    event_cursor
                } else {
                    0
                };
                s.last_id = cursor;
                s.tick = 1;
                let event = Event::default()
                    .event("init")
                    .data(json!({ "project_id": s.project_id, "cursor": cursor }).to_string());
                Some((Ok(event), s))
            }
            tick if tick % 15 == 0 => {
                // 每 30s 心跳
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                s.tick += 1;
                let event = Event::default()
                    .event("heartbeat")
                    .data(json!({ "timestamp": Utc::now().timestamp_millis() }).to_string());
                Some((Ok(event), s))
            }
            _ => {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                s.tick += 1;

                if let Some(db) = &s.db {
                    let mut q = models::TrackEvent::find()
                        .filter(models::track_event::Column::ProjectId.eq(s.project_id))
                        .filter(models::track_event::Column::Id.gt(s.last_id))
                        .order_by_asc(models::track_event::Column::Id)
                        .limit(5);

                    if let Some(did) = &s.distinct_id {
                        q = q.filter(models::track_event::Column::DistinctId.eq(did.as_str()));
                    }

                    if let Ok(events) = q.all(db).await {
                        if !events.is_empty() {
                            if let Some(max_id) = events.iter().map(|e| e.id).max() {
                                s.last_id = max_id;
                            }
                            let e = &events[0];
                            let event = Event::default().event("track").data(
                                json!({
                                    "id": e.id,
                                    "event": e.event,
                                    "distinct_id": e.distinct_id,
                                    "properties": e.properties,
                                    "page_url": e.page_url,
                                    "created_at": e.created_at,
                                })
                                .to_string(),
                            );
                            return Some((Ok(event), s));
                        }
                    }

                    let mut profile_query = models::TrackUserProfile::find()
                        .filter(models::track_user_profile::Column::ProjectId.eq(s.project_id))
                        .order_by_asc(models::track_user_profile::Column::UpdatedAt)
                        .limit(5);

                    if let Some(last_updated) = s.last_profile_updated_at {
                        profile_query = profile_query
                            .filter(models::track_user_profile::Column::UpdatedAt.gt(last_updated));
                    }
                    if let Some(did) = &s.distinct_id {
                        profile_query = profile_query.filter(
                            models::track_user_profile::Column::DistinctId.eq(did.as_str()),
                        );
                    }

                    if let Ok(profiles) = profile_query.all(db).await {
                        if !profiles.is_empty() {
                            if let Some(max_updated) = profiles.iter().map(|p| p.updated_at).max() {
                                s.last_profile_updated_at = Some(max_updated);
                            }
                            let p = &profiles[0];
                            let event = Event::default().event("profile").data(
                                json!({
                                    "id": p.id,
                                    "distinct_id": p.distinct_id,
                                    "anonymous_id": p.anonymous_id,
                                    "user_id": p.user_id,
                                    "properties": p.properties,
                                    "updated_at": p.updated_at,
                                })
                                .to_string(),
                            );
                            return Some((Ok(event), s));
                        }
                    }
                }
                let event = Event::default().comment("poll");
                Some((Ok(event), s))
            }
        }
    });

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, TimeZone, Timelike};

    fn mock_track_event(
        event: &str,
        browser: Option<&str>,
        os: Option<&str>,
    ) -> models::track_event::Model {
        models::track_event::Model {
            id: 1,
            project_id: 1,
            app_id: "".into(),
            distinct_id: "".into(),
            anonymous_id: None,
            user_id: None,
            is_login_id: false,
            event: event.into(),
            event_type: "".into(),
            properties: None,
            super_properties: None,
            session_id: None,
            event_duration: None,
            page_url: None,
            page_title: None,
            referrer: None,
            viewport: None,
            screen_resolution: None,
            user_agent: None,
            browser: browser.map(|s| s.into()),
            browser_version: None,
            os: os.map(|s| s.into()),
            os_version: None,
            device_type: None,
            language: None,
            timezone: None,
            sdk_version: None,
            release: None,
            environment: None,
            client_time: None,
            created_at: chrono::FixedOffset::east_opt(0)
                .unwrap()
                .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
                .unwrap(),
        }
    }

    #[test]
    fn test_default_days() {
        assert_eq!(default_days(), 7);
    }

    #[test]
    fn test_default_metric() {
        assert_eq!(default_metric(), "pv");
    }

    #[test]
    fn test_row_date() {
        let mut row = mock_track_event("", None, None);
        row.created_at = chrono::FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2024, 1, 15, 10, 30, 0)
            .unwrap();
        assert_eq!(row_date(&row), "2024-01-15");
    }

    #[test]
    fn test_dim_value() {
        let mut row = mock_track_event("", None, None);
        row.browser = Some("Chrome".into());
        row.os = Some("macOS".into());
        row.device_type = Some("desktop".into());
        row.environment = Some("production".into());
        assert_eq!(dim_value(&row, "browser"), "Chrome");
        assert_eq!(dim_value(&row, "os"), "macOS");
        assert_eq!(dim_value(&row, "device_type"), "desktop");
        assert_eq!(dim_value(&row, "environment"), "production");
        assert_eq!(dim_value(&row, "unknown"), "Unknown");
    }

    #[test]
    fn test_parse_time_bound_rfc3339() {
        let result = parse_time_bound(Some("2024-01-15T10:30:00+00:00"), false);
        assert!(result.is_some());
        let dt = result.unwrap();
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 10);
    }

    #[test]
    fn test_parse_time_bound_date() {
        let result = parse_time_bound(Some("2024-01-15"), false);
        assert!(result.is_some());
        let dt = result.unwrap();
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 0);
    }

    #[test]
    fn test_parse_time_bound_empty() {
        assert!(parse_time_bound(Some(""), false).is_none());
        assert!(parse_time_bound(None, false).is_none());
    }

    #[test]
    fn test_json_value_matches_string() {
        assert!(json_value_matches(&json!("hello"), &json!("hello")));
        assert!(!json_value_matches(&json!("hello"), &json!("world")));
    }

    #[test]
    fn test_json_value_matches_number() {
        assert!(json_value_matches(&json!(42), &json!(42)));
        assert!(json_value_matches(&json!(42.0), &json!(42)));
        assert!(!json_value_matches(&json!(42), &json!(43)));
    }

    #[test]
    fn test_json_value_matches_bool() {
        assert!(json_value_matches(&json!(true), &json!(true)));
        assert!(!json_value_matches(&json!(true), &json!(false)));
    }

    #[test]
    fn test_json_value_matches_null() {
        assert!(json_value_matches(&json!(null), &json!(null)));
        assert!(!json_value_matches(&json!("x"), &json!(null)));
    }

    #[test]
    fn test_json_value_matches_array() {
        assert!(json_value_matches(&json!("b"), &json!(["a", "b", "c"])));
        assert!(!json_value_matches(&json!("d"), &json!(["a", "b", "c"])));
    }

    #[test]
    fn test_json_lookup_found() {
        let source = Some(json!({"name": "test", "age": 30}));
        assert_eq!(json_lookup(&source, "name"), Some(json!("test")));
    }

    #[test]
    fn test_json_lookup_missing() {
        let source = Some(json!({"name": "test"}));
        assert_eq!(json_lookup(&source, "missing"), None);
    }

    #[test]
    fn test_event_field_value_event() {
        let row = mock_track_event("click", Some("Chrome"), None);
        assert_eq!(event_field_value(&row, "event"), Some(json!("click")));
        assert_eq!(event_field_value(&row, "browser"), Some(json!("Chrome")));
    }

    #[test]
    fn test_event_matches_filters_empty() {
        let row = mock_track_event("click", None, None);
        assert!(event_matches_filters(&row, None));
        assert!(event_matches_filters(&row, Some(&json!({}))));
    }

    #[test]
    fn test_dim_or_property_value() {
        let row = mock_track_event("click", Some("Chrome"), None);
        assert_eq!(dim_or_property_value(&row, "event"), "click");
        assert_eq!(dim_or_property_value(&row, "browser"), "Chrome");
        assert_eq!(dim_or_property_value(&row, "unknown"), "Unknown");
    }

    #[test]
    fn test_default_window() {
        assert_eq!(default_window(), 1440);
    }

    #[test]
    fn test_default_retention_days() {
        assert_eq!(default_retention_days(), 7);
    }

    #[test]
    fn test_retention_bucket() {
        use chrono::NaiveDate;
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        assert_eq!(retention_bucket(date, "day"), date);
        assert_eq!(retention_bucket(date, "week"), NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
        // month type not implemented in retention_bucket, falls through to date
    }
}
