//! 埋点事件管理（Phase 3）。
//!
//! 已采集事件（来自 track_events）：
//!   GET  /api/track/events               列表（含统计数据）
//!   GET  /api/track/events/:name         详情（属性示例 + 7 天趋势）
//!
//! 自定义事件定义（track_event_definitions 表）：
//!   GET    /api/track/definitions         列表
//!   POST   /api/track/definitions         创建
//!   PUT    /api/track/definitions/:id     更新
//!   DELETE /api/track/definitions/:id     删除
//!
//! 属性管理（聚合自 definitions 中的 properties 字段）：
//!   GET  /api/track/properties            该项目所有属性（去重）

use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use chrono::{DateTime, Duration, FixedOffset, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

use crate::error::{AppError, AppResult};
use crate::middleware::auth::{check_project_access, CurrentUser};
use crate::models;
use crate::router::AppState;

fn now_fixed() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap())
}

// ════════════════════════════════════════════════════════════════════
// 已采集事件列表 / 详情
// ════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct EventListQuery {
    pub project_id: i32,
    #[serde(default)]
    pub keyword: Option<String>,
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
}

fn default_page() -> u64 {
    1
}
fn default_page_size() -> u64 {
    20
}

pub async fn list_events(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Query(q): Query<EventListQuery>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    check_project_access(db, &current_user, q.project_id).await?;
    let since = (Utc::now() - Duration::days(90)).with_timezone(&FixedOffset::east_opt(0).unwrap());

    let rows = models::TrackEvent::find()
        .filter(models::track_event::Column::ProjectId.eq(q.project_id))
        .filter(models::track_event::Column::CreatedAt.gte(since))
        .order_by_desc(models::track_event::Column::CreatedAt)
        .limit(5000)
        .all(db)
        .await?;

    let mut event_map: HashMap<String, (u64, String, HashSet<String>)> = HashMap::new();
    for row in &rows {
        let entry = event_map
            .entry(row.event.clone())
            .or_insert_with(|| (0, row.created_at.to_rfc3339(), HashSet::new()));
        entry.0 += 1;
        entry.2.insert(row.distinct_id.clone());
    }

    let mut events: Vec<Value> = event_map
        .into_iter()
        .filter(|(name, _)| {
            q.keyword
                .as_ref()
                .map(|kw| name.contains(kw.as_str()))
                .unwrap_or(true)
        })
        .map(|(name, (count, last_seen, users))| {
            let category = if name.starts_with('$') {
                "auto"
            } else {
                "custom"
            };
            json!({
                "event": name,
                "category": category,
                "total_count": count,
                "unique_users": users.len(),
                "last_seen": last_seen,
            })
        })
        .collect();

    events.sort_by(|a, b| {
        let va = a["total_count"].as_u64().unwrap_or(0);
        let vb = b["total_count"].as_u64().unwrap_or(0);
        vb.cmp(&va)
    });

    let page_size = q.page_size.clamp(1, 100);
    let total = events.len() as u64;
    let start = ((q.page.saturating_sub(1)) * page_size) as usize;
    let page_events: Vec<Value> = events
        .into_iter()
        .skip(start)
        .take(page_size as usize)
        .collect();

    Ok(Json(json!({
        "code": 0, "message": "ok",
        "data": { "list": page_events, "total": total },
        "pagination": {
            "page": q.page, "page_size": page_size,
            "total": total,
            "total_pages": (total as f64 / page_size as f64).ceil() as u64
        }
    })))
}

#[derive(Debug, Deserialize)]
pub struct EventDetailQuery {
    pub project_id: i32,
}

pub async fn event_detail(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(event_name): Path<String>,
    Query(q): Query<EventDetailQuery>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    check_project_access(db, &current_user, q.project_id).await?;
    let since = (Utc::now() - Duration::days(7)).with_timezone(&FixedOffset::east_opt(0).unwrap());

    let rows = models::TrackEvent::find()
        .filter(models::track_event::Column::ProjectId.eq(q.project_id))
        .filter(models::track_event::Column::Event.eq(event_name.clone()))
        .filter(models::track_event::Column::CreatedAt.gte(since))
        .order_by_desc(models::track_event::Column::CreatedAt)
        .limit(500)
        .all(db)
        .await?;

    let mut prop_keys: HashSet<String> = HashSet::new();
    for row in &rows {
        if let Some(Value::Object(props)) = &row.properties {
            for k in props.keys() {
                prop_keys.insert(k.clone());
            }
        }
    }

    let mut trend = Vec::new();
    for i in (0..7).rev() {
        let day = (Utc::now() - Duration::days(i)).date_naive();
        let s = day
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .with_timezone(&FixedOffset::east_opt(0).unwrap());
        let e = (day + chrono::Days::new(1))
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .with_timezone(&FixedOffset::east_opt(0).unwrap());
        let count = rows
            .iter()
            .filter(|r| r.created_at >= s && r.created_at < e)
            .count();
        trend.push(json!({ "date": day.to_string(), "count": count }));
    }

    let unique_users: HashSet<&str> = rows.iter().map(|r| r.distinct_id.as_str()).collect();

    Ok(Json(json!({
        "code": 0, "message": "ok",
        "data": {
            "event": event_name,
            "total_count": rows.len(),
            "unique_users": unique_users.len(),
            "properties": prop_keys.into_iter().collect::<Vec<_>>(),
            "trend": trend,
        }
    })))
}

// ════════════════════════════════════════════════════════════════════
// 自定义事件定义 CRUD
// ════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct DefListQuery {
    pub project_id: i32,
    #[serde(default)]
    pub keyword: Option<String>,
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
}

#[derive(Debug, Deserialize)]
pub struct CreateDefBody {
    pub project_id: i32,
    pub event_name: String,
    pub display_name: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    /// 属性定义列表：[{name, type, description, required}]
    pub properties: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDefBody {
    pub display_name: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub properties: Option<Value>,
    pub status: Option<String>,
}

pub async fn list_definitions(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Query(q): Query<DefListQuery>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    check_project_access(db, &current_user, q.project_id).await?;
    let mut cursor = models::EventDefinition::find()
        .filter(models::event_definition::Column::ProjectId.eq(q.project_id));

    if let Some(kw) = q.keyword.as_ref().filter(|s| !s.is_empty()) {
        cursor = cursor.filter(models::event_definition::Column::EventName.contains(kw));
    }

    let page_size = q.page_size.clamp(1, 100);
    let total = cursor.clone().count(db).await? as u64;

    let items = cursor
        .order_by_desc(models::event_definition::Column::Id)
        .offset(Some((q.page.saturating_sub(1)) * page_size))
        .limit(Some(page_size))
        .all(db)
        .await?;

    Ok(Json(json!({
        "code": 0, "message": "ok",
        "data": { "list": items, "total": total },
        "pagination": {
            "page": q.page, "page_size": page_size, "total": total,
            "total_pages": (total as f64 / page_size as f64).ceil() as u64
        }
    })))
}

pub async fn create_definition(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(body): Json<CreateDefBody>,
) -> AppResult<Json<Value>> {
    if body.event_name.trim().is_empty() {
        return Err(AppError::BadRequest("event_name is required".into()));
    }
    let db = get_db(&state)?;
    check_project_access(db, &current_user, body.project_id).await?;

    let existing = models::EventDefinition::find()
        .filter(models::event_definition::Column::ProjectId.eq(body.project_id))
        .filter(models::event_definition::Column::EventName.eq(body.event_name.clone()))
        .one(db)
        .await?;
    if existing.is_some() {
        return Err(AppError::BadRequest(format!("event '{}' already defined", body.event_name)));
    }

    let active = models::event_definition::ActiveModel {
        id: sea_orm::NotSet,
        project_id: Set(body.project_id),
        event_name: Set(body.event_name),
        display_name: Set(body.display_name),
        category: Set(body.category),
        description: Set(body.description),
        properties: Set(body.properties),
        status: Set("active".into()),
        created_at: Set(now_fixed()),
        updated_at: Set(now_fixed()),
    };
    let created = active.insert(db).await?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": created })))
}

pub async fn update_definition(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i32>,
    Json(body): Json<UpdateDefBody>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let row = models::EventDefinition::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    check_project_access(db, &current_user, row.project_id).await?;

    let mut am: models::event_definition::ActiveModel = row.into();
    if let Some(v) = body.display_name {
        am.display_name = Set(Some(v));
    }
    if let Some(v) = body.category {
        am.category = Set(Some(v));
    }
    if let Some(v) = body.description {
        am.description = Set(Some(v));
    }
    if let Some(v) = body.properties {
        am.properties = Set(Some(v));
    }
    if let Some(v) = body.status {
        am.status = Set(v);
    }
    am.updated_at = Set(now_fixed());

    let updated = am.update(db).await?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": updated })))
}

pub async fn delete_definition(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i32>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    let row = models::EventDefinition::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)?;
    check_project_access(db, &current_user, row.project_id).await?;
    let res = models::EventDefinition::delete_by_id(id).exec(db).await?;
    if res.rows_affected == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(json!({ "code": 0, "message": "ok", "data": { "deleted": id } })))
}

// ════════════════════════════════════════════════════════════════════
// 属性管理：汇总所有已定义事件的属性（去重）
// ════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct PropQuery {
    pub project_id: i32,
}

#[derive(Debug, Serialize)]
pub struct PropItem {
    pub name: String,
    pub prop_type: String,
    pub description: Option<String>,
    pub required: bool,
    pub event_names: Vec<String>,
}

pub async fn list_properties(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    Query(q): Query<PropQuery>,
) -> AppResult<Json<Value>> {
    let db = get_db(&state)?;
    check_project_access(db, &current_user, q.project_id).await?;
    let defs = models::EventDefinition::find()
        .filter(models::event_definition::Column::ProjectId.eq(q.project_id))
        .filter(models::event_definition::Column::Status.eq("active"))
        .limit(500)
        .all(db)
        .await?;

    // 汇总：prop_name → PropItem
    let mut prop_map: HashMap<String, PropItem> = HashMap::new();
    for def in &defs {
        if let Some(Value::Array(props)) = &def.properties {
            for p in props {
                let name = p
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if name.is_empty() {
                    continue;
                }
                let entry = prop_map.entry(name.clone()).or_insert_with(|| PropItem {
                    name: name.clone(),
                    prop_type: p
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("string")
                        .to_string(),
                    description: p
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    required: p.get("required").and_then(|v| v.as_bool()).unwrap_or(false),
                    event_names: vec![],
                });
                entry.event_names.push(def.event_name.clone());
            }
        }
    }

    let mut result: Vec<&PropItem> = prop_map.values().collect();
    result.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(Json(json!({
        "code": 0, "message": "ok",
        "data": { "list": result, "total": result.len() }
    })))
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
    fn test_event_list_query_defaults() {
        let q: EventListQuery = serde_json::from_str(r#"{"project_id":1}"#).unwrap();
        assert_eq!(q.project_id, 1);
        assert!(q.keyword.is_none());
        assert_eq!(q.page, 1);
        assert_eq!(q.page_size, 20);
    }

    #[test]
    fn test_event_list_query_with_keyword() {
        let q: EventListQuery =
            serde_json::from_str(r#"{"project_id":1,"keyword":"click","page":2}"#).unwrap();
        assert_eq!(q.keyword.as_deref(), Some("click"));
        assert_eq!(q.page, 2);
    }

    #[test]
    fn test_event_detail_query() {
        let q: EventDetailQuery = serde_json::from_str(r#"{"project_id":1}"#).unwrap();
        assert_eq!(q.project_id, 1);
    }

    #[test]
    fn test_def_list_query_defaults() {
        let q: DefListQuery = serde_json::from_str(r#"{"project_id":1}"#).unwrap();
        assert_eq!(q.project_id, 1);
        assert!(q.keyword.is_none());
        assert_eq!(q.page, 1);
    }

    #[test]
    fn test_create_def_body() {
        let json_str = r#"{"project_id":1,"event_name":"page_view","display_name":"Page View"}"#;
        let body: CreateDefBody = serde_json::from_str(json_str).unwrap();
        assert_eq!(body.event_name, "page_view");
        assert_eq!(body.display_name.as_deref(), Some("Page View"));
        assert!(body.category.is_none());
    }

    #[test]
    fn test_update_def_body_all_optional() {
        let json_str = r#"{}"#;
        let body: UpdateDefBody = serde_json::from_str(json_str).unwrap();
        assert!(body.display_name.is_none());
        assert!(body.category.is_none());
        assert!(body.description.is_none());
        assert!(body.properties.is_none());
        assert!(body.status.is_none());
    }

    #[test]
    fn test_prop_query() {
        let q: PropQuery = serde_json::from_str(r#"{"project_id":1}"#).unwrap();
        assert_eq!(q.project_id, 1);
    }

    #[test]
    fn test_now_fixed_is_utc() {
        let now = now_fixed();
        assert_eq!(now.offset().local_minus_utc(), 0);
    }

    #[test]
    fn test_prop_item_serialization() {
        let item = PropItem {
            name: "page_url".into(),
            prop_type: "string".into(),
            description: Some("The page URL".into()),
            required: true,
            event_names: vec!["page_view".into()],
        };
        let json_str = serde_json::to_string(&item).unwrap();
        assert!(json_str.contains("page_url"));
        assert!(json_str.contains("page_view"));
    }
}
