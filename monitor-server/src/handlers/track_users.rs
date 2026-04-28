//! 埋点用户画像查询。

use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use chrono::{DateTime, FixedOffset};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::middleware::auth::{check_project_access, CurrentUser};
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
    Extension(current_user): Extension<CurrentUser>,
    Query(q): Query<UserListQuery>,
) -> AppResult<Json<Value>> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::Internal("database not connected".into()))?;
    check_project_access(db, &current_user, q.project_id).await?;

    let filters = parse_filters(q.filters.as_deref())?;
    let keyword = q.keyword.unwrap_or_default().trim().to_lowercase();

    // 数据库级关键词过滤（避免全表加载）
    let mut cursor = models::TrackUserProfile::find()
        .filter(models::track_user_profile::Column::ProjectId.eq(q.project_id));

    if !keyword.is_empty() {
        let like_pattern = format!("%{}%", keyword);
        cursor = cursor.filter(
            models::track_user_profile::Column::DistinctId
                .contains(&like_pattern)
                .or(models::track_user_profile::Column::UserId.contains(&like_pattern))
                .or(models::track_user_profile::Column::Name.contains(&like_pattern))
                .or(models::track_user_profile::Column::Email.contains(&like_pattern)),
        );
    }

    // 按 last_visit_at 降序排序，数据库级分页
    let page_size = q.page_size.clamp(1, 100);
    let paginator = cursor
        .order_by_desc(models::track_user_profile::Column::LastVisitAt)
        .paginate(db, page_size);
    let total = paginator.num_items().await?;
    let rows = paginator.fetch_page(q.page.saturating_sub(1)).await?;

    // 仅对当前页数据做应用层属性过滤（避免全表加载）
    let filtered: Vec<TrackUserDto> = rows
        .into_iter()
        .filter(|row| {
            filters
                .iter()
                .all(|filter| property_matches(&row.properties, filter))
        })
        .map(Into::into)
        .collect();

    Ok(Json(json!({
        "code": 0,
        "message": "ok",
        "data": { "list": filtered, "total": total },
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
    Path(distinct_id): Path<String>,
    Query(q): Query<UserDetailQuery>,
) -> AppResult<Json<Value>> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::Internal("database not connected".into()))?;
    check_project_access(db, &current_user, q.project_id).await?;

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
    Extension(current_user): Extension<CurrentUser>,
    Path(distinct_id): Path<String>,
    Query(q): Query<UserEventsQuery>,
) -> AppResult<Json<Value>> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::Internal("database not connected".into()))?;
    check_project_access(db, &current_user, q.project_id).await?;

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

    let page_size = q.page_size.clamp(1, 100);
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

#[cfg(test)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_list_query_defaults() {
        let q: UserListQuery = serde_json::from_str(r#"{"project_id":1}"#).unwrap();
        assert_eq!(q.project_id, 1);
        assert_eq!(q.page, 1);
        assert_eq!(q.page_size, 20);
        assert!(q.keyword.is_none());
        assert!(q.filters.is_none());
    }

    #[test]
    fn test_user_list_query_custom() {
        let q: UserListQuery =
            serde_json::from_str(r#"{"project_id":1,"page":2,"page_size":50,"keyword":"test"}"#)
                .unwrap();
        assert_eq!(q.page, 2);
        assert_eq!(q.page_size, 50);
        assert_eq!(q.keyword.as_deref(), Some("test"));
    }

    #[test]
    fn test_user_events_query_defaults() {
        let q: UserEventsQuery = serde_json::from_str(r#"{"project_id":1}"#).unwrap();
        assert_eq!(q.page, 1);
        assert_eq!(q.page_size, 20);
        assert!(q.event_name.is_none());
    }

    #[test]
    fn test_property_matches_eq() {
        let filter = PropertyFilter {
            property: "name".into(),
            operator: "eq".into(),
            value: Some(json!("Alice")),
        };
        let props = json!({"name": "Alice"});
        assert!(property_matches(&props, &filter));
    }

    #[test]
    fn test_property_matches_eq_not_equal() {
        let filter = PropertyFilter {
            property: "name".into(),
            operator: "eq".into(),
            value: Some(json!("Alice")),
        };
        let props = json!({"name": "Bob"});
        assert!(!property_matches(&props, &filter));
    }

    #[test]
    fn test_property_matches_neq() {
        let filter = PropertyFilter {
            property: "name".into(),
            operator: "neq".into(),
            value: Some(json!("Alice")),
        };
        let props = json!({"name": "Bob"});
        assert!(property_matches(&props, &filter));
    }

    #[test]
    fn test_property_matches_contains() {
        let filter = PropertyFilter {
            property: "name".into(),
            operator: "contains".into(),
            value: Some(json!("li")),
        };
        let props = json!({"name": "Alice"});
        assert!(property_matches(&props, &filter));
    }

    #[test]
    fn test_property_matches_contains_case_insensitive() {
        let filter = PropertyFilter {
            property: "name".into(),
            operator: "contains".into(),
            value: Some(json!("ALICE")),
        };
        let props = json!({"name": "alice"});
        assert!(property_matches(&props, &filter));
    }

    #[test]
    fn test_property_matches_exists() {
        let filter =
            PropertyFilter { property: "name".into(), operator: "exists".into(), value: None };
        let props = json!({"name": "Alice"});
        assert!(property_matches(&props, &filter));
    }

    #[test]
    fn test_property_matches_not_exists() {
        let filter =
            PropertyFilter { property: "age".into(), operator: "not_exists".into(), value: None };
        let props = json!({"name": "Alice"});
        assert!(property_matches(&props, &filter));
    }

    #[test]
    fn test_property_matches_missing_property() {
        let filter = PropertyFilter {
            property: "age".into(),
            operator: "eq".into(),
            value: Some(json!(30)),
        };
        let props = json!({"name": "Alice"});
        assert!(!property_matches(&props, &filter));
    }

    #[test]
    fn test_value_to_string_string() {
        assert_eq!(value_to_string(&json!("hello")), Some("hello".into()));
    }

    #[test]
    fn test_value_to_string_number() {
        assert_eq!(value_to_string(&json!(42)), Some("42".into()));
    }

    #[test]
    fn test_value_to_string_bool() {
        assert_eq!(value_to_string(&json!(true)), Some("true".into()));
    }

    #[test]
    fn test_value_to_string_array() {
        assert_eq!(value_to_string(&json!(["a", "b"])), Some("a,b".into()));
    }

    #[test]
    fn test_value_to_string_null() {
        assert_eq!(value_to_string(&json!(null)), None);
    }

    #[test]
    fn test_parse_filters_empty() {
        let filters = parse_filters(None).unwrap();
        assert!(filters.is_empty());
    }

    #[test]
    fn test_parse_filters_empty_string() {
        let filters = parse_filters(Some("")).unwrap();
        assert!(filters.is_empty());
    }

    #[test]
    fn test_parse_filters_valid() {
        let raw = r#"[{"property":"name","operator":"eq","value":"test"}]"#;
        let filters = parse_filters(Some(raw)).unwrap();
        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0].property, "name");
    }

    #[test]
    fn test_parse_filters_invalid_json() {
        let result = parse_filters(Some("not json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_time_empty() {
        assert!(parse_time(None).unwrap().is_none());
        assert!(parse_time(Some("")).unwrap().is_none());
        assert!(parse_time(Some("  ")).unwrap().is_none());
    }

    #[test]
    fn test_parse_time_valid() {
        let dt = parse_time(Some("2024-01-15T10:30:00Z")).unwrap();
        assert!(dt.is_some());
    }

    #[test]
    fn test_parse_time_invalid() {
        let result = parse_time(Some("not-a-date"));
        assert!(result.is_err());
    }

    #[test]
    fn test_keyword_matches_empty() {
        use chrono::Utc;
        let row = models::track_user_profile::Model {
            id: 1,
            project_id: 1,
            distinct_id: "user1".into(),
            anonymous_id: None,
            user_id: None,
            name: None,
            email: None,
            phone: None,
            properties: json!({}),
            first_visit_at: None,
            last_visit_at: None,
            total_events: 0,
            total_sessions: 0,
            created_at: Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap()),
            updated_at: Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap()),
        };
        assert!(keyword_matches(&row, ""));
    }

    #[test]
    fn test_keyword_matches_distinct_id() {
        use chrono::Utc;
        let row = models::track_user_profile::Model {
            id: 1,
            project_id: 1,
            distinct_id: "user_abc".into(),
            anonymous_id: None,
            user_id: None,
            name: None,
            email: None,
            phone: None,
            properties: json!({}),
            first_visit_at: None,
            last_visit_at: None,
            total_events: 0,
            total_sessions: 0,
            created_at: Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap()),
            updated_at: Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap()),
        };
        assert!(keyword_matches(&row, "abc"));
        assert!(!keyword_matches(&row, "xyz"));
    }
}
