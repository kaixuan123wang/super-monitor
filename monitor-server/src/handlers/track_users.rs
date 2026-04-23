//! 埋点用户画像查询。

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, FixedOffset};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::models;
use crate::router::AppState;

#[derive(Debug, Deserialize)]
pub struct UserListQuery {
    pub project_id: i32,
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
    pub keyword: Option<String>,
    pub filters: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UserDetailQuery {
    pub project_id: i32,
}

#[derive(Debug, Deserialize)]
pub struct UserEventsQuery {
    pub project_id: i32,
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
    pub event_name: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PropertyFilter {
    property: String,
    #[serde(default = "default_operator")]
    operator: String,
    value: Option<Value>,
}

#[derive(Debug, Serialize)]
struct TrackUserDto {
    id: i64,
    project_id: i32,
    distinct_id: String,
    anonymous_id: Option<String>,
    user_id: Option<String>,
    name: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    properties: Value,
    first_visit_at: Option<DateTime<FixedOffset>>,
    last_visit_at: Option<DateTime<FixedOffset>>,
    total_events: i32,
    total_sessions: i32,
    created_at: DateTime<FixedOffset>,
    updated_at: DateTime<FixedOffset>,
}

#[derive(Debug, Serialize)]
struct TrackEventDto {
    id: i64,
    project_id: i32,
    distinct_id: String,
    anonymous_id: Option<String>,
    user_id: Option<String>,
    is_login_id: bool,
    event: String,
    event_type: String,
    properties: Option<Value>,
    super_properties: Option<Value>,
    session_id: Option<String>,
    page_url: Option<String>,
    page_title: Option<String>,
    referrer: Option<String>,
    browser: Option<String>,
    os: Option<String>,
    device_type: Option<String>,
    client_time: Option<DateTime<FixedOffset>>,
    created_at: DateTime<FixedOffset>,
}

fn default_page() -> u64 {
    1
}

fn default_page_size() -> u64 {
    20
}

fn default_operator() -> String {
    "eq".into()
}

pub async fn list(
    State(state): State<AppState>,
    Query(q): Query<UserListQuery>,
) -> AppResult<Json<Value>> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::Internal("database not connected".into()))?;

    let filters = parse_filters(q.filters.as_deref())?;
    let keyword = q.keyword.unwrap_or_default().trim().to_lowercase();

    let rows = models::TrackUserProfile::find()
        .filter(models::track_user_profile::Column::ProjectId.eq(q.project_id))
        .order_by_desc(models::track_user_profile::Column::LastVisitAt)
        .all(db)
        .await?;

    let mut matched: Vec<models::track_user_profile::Model> = rows
        .into_iter()
        .filter(|row| keyword_matches(row, &keyword))
        .filter(|row| {
            filters
                .iter()
                .all(|filter| property_matches(&row.properties, filter))
        })
        .collect();

    matched.sort_by(|a, b| b.last_visit_at.cmp(&a.last_visit_at));

    let total = matched.len() as u64;
    let page_size = q.page_size.max(1);
    let start = q.page.saturating_sub(1).saturating_mul(page_size) as usize;
    let list: Vec<TrackUserDto> = matched
        .into_iter()
        .skip(start)
        .take(page_size as usize)
        .map(Into::into)
        .collect();

    Ok(Json(json!({
        "code": 0,
        "message": "ok",
        "data": { "list": list, "total": total },
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
    Path(distinct_id): Path<String>,
    Query(q): Query<UserDetailQuery>,
) -> AppResult<Json<Value>> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::Internal("database not connected".into()))?;

    let user = models::TrackUserProfile::find()
        .filter(models::track_user_profile::Column::ProjectId.eq(q.project_id))
        .filter(models::track_user_profile::Column::DistinctId.eq(distinct_id.clone()))
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;

    let events = models::TrackEvent::find()
        .filter(models::track_event::Column::ProjectId.eq(q.project_id))
        .filter(models::track_event::Column::DistinctId.eq(distinct_id))
        .order_by_desc(models::track_event::Column::CreatedAt)
        .limit(20)
        .all(db)
        .await?;

    let recent_events: Vec<TrackEventDto> = events.into_iter().map(Into::into).collect();

    Ok(Json(json!({
        "code": 0,
        "message": "ok",
        "data": {
            "user": TrackUserDto::from(user),
            "recent_events": recent_events
        }
    })))
}

pub async fn events(
    State(state): State<AppState>,
    Path(distinct_id): Path<String>,
    Query(q): Query<UserEventsQuery>,
) -> AppResult<Json<Value>> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::Internal("database not connected".into()))?;

    let mut cursor = models::TrackEvent::find()
        .filter(models::track_event::Column::ProjectId.eq(q.project_id))
        .filter(models::track_event::Column::DistinctId.eq(distinct_id));

    if let Some(event_name) = q.event_name.as_ref().filter(|s| !s.trim().is_empty()) {
        cursor = cursor.filter(models::track_event::Column::Event.eq(event_name.trim()));
    }
    if let Some(start) = parse_time(q.start_time.as_deref())? {
        cursor = cursor.filter(models::track_event::Column::CreatedAt.gte(start));
    }
    if let Some(end) = parse_time(q.end_time.as_deref())? {
        cursor = cursor.filter(models::track_event::Column::CreatedAt.lte(end));
    }

    let page_size = q.page_size.max(1);
    let paginator = cursor
        .order_by_desc(models::track_event::Column::CreatedAt)
        .paginate(db, page_size);
    let total = paginator.num_items().await?;
    let rows = paginator.fetch_page(q.page.saturating_sub(1)).await?;
    let list: Vec<TrackEventDto> = rows.into_iter().map(Into::into).collect();

    Ok(Json(json!({
        "code": 0,
        "message": "ok",
        "data": { "list": list, "total": total },
        "pagination": {
            "page": q.page,
            "page_size": page_size,
            "total": total,
            "total_pages": (total as f64 / page_size as f64).ceil() as u64
        }
    })))
}

fn parse_filters(raw: Option<&str>) -> AppResult<Vec<PropertyFilter>> {
    let Some(raw) = raw.filter(|s| !s.trim().is_empty()) else {
        return Ok(vec![]);
    };
    serde_json::from_str(raw)
        .map_err(|_| AppError::BadRequest("filters must be a JSON array".into()))
}

fn keyword_matches(row: &models::track_user_profile::Model, keyword: &str) -> bool {
    if keyword.is_empty() {
        return true;
    }
    row.distinct_id.to_lowercase().contains(keyword)
        || row
            .user_id
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains(keyword)
        || row
            .name
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains(keyword)
        || row
            .email
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains(keyword)
}

fn property_matches(properties: &Value, filter: &PropertyFilter) -> bool {
    let Some(actual) = properties.get(&filter.property) else {
        return filter.operator == "not_exists";
    };

    match filter.operator.as_str() {
        "eq" => filter.value.as_ref().map(|v| actual == v).unwrap_or(false),
        "neq" => filter.value.as_ref().map(|v| actual != v).unwrap_or(true),
        "contains" => {
            let expected = filter
                .value
                .as_ref()
                .and_then(value_to_string)
                .unwrap_or_default()
                .to_lowercase();
            value_to_string(actual)
                .map(|v| v.to_lowercase().contains(&expected))
                .unwrap_or(false)
        }
        "exists" => true,
        "not_exists" => false,
        _ => false,
    }
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Array(items) => Some(
            items
                .iter()
                .filter_map(value_to_string)
                .collect::<Vec<_>>()
                .join(","),
        ),
        _ => None,
    }
}

fn parse_time(raw: Option<&str>) -> AppResult<Option<DateTime<FixedOffset>>> {
    let Some(raw) = raw.filter(|s| !s.trim().is_empty()) else {
        return Ok(None);
    };
    DateTime::parse_from_rfc3339(raw)
        .map(Some)
        .map_err(|_| AppError::BadRequest("time must be RFC3339 format".into()))
}

impl From<models::track_user_profile::Model> for TrackUserDto {
    fn from(row: models::track_user_profile::Model) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            distinct_id: row.distinct_id,
            anonymous_id: row.anonymous_id,
            user_id: row.user_id,
            name: row.name,
            email: row.email,
            phone: row.phone,
            properties: row.properties,
            first_visit_at: row.first_visit_at,
            last_visit_at: row.last_visit_at,
            total_events: row.total_events,
            total_sessions: row.total_sessions,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<models::track_event::Model> for TrackEventDto {
    fn from(row: models::track_event::Model) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            distinct_id: row.distinct_id,
            anonymous_id: row.anonymous_id,
            user_id: row.user_id,
            is_login_id: row.is_login_id,
            event: row.event,
            event_type: row.event_type,
            properties: row.properties,
            super_properties: row.super_properties,
            session_id: row.session_id,
            page_url: row.page_url,
            page_title: row.page_title,
            referrer: row.referrer,
            browser: row.browser,
            os: row.os,
            device_type: row.device_type,
            client_time: row.client_time,
            created_at: row.created_at,
        }
    }
}
