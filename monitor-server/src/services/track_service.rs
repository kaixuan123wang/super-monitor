//! 埋点数据处理：写入 track_events / track_user_profiles。

use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use sea_orm::prelude::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde_json::{json, Value};
use std::str::FromStr;

use crate::error::{AppError, AppResult};
use crate::models;

fn now_fixed() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap())
}

fn data_str(data: &Value, key: &str) -> Option<String> {
    data.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn ctx_str(ctx: &Value, key: &str) -> Option<String> {
    ctx.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn parse_client_time(data: &Value) -> Option<DateTime<FixedOffset>> {
    data.get("client_time")
        .and_then(|v| v.as_i64())
        .and_then(|millis| {
            Utc.timestamp_millis_opt(millis)
                .single()
                .map(|dt| dt.with_timezone(&FixedOffset::east_opt(0).unwrap()))
        })
}

fn decimal_from_json(value: &Value) -> Option<Decimal> {
    if let Some(n) = value.as_f64() {
        return Decimal::from_str(&format!("{n:.3}")).ok();
    }
    value.as_str().and_then(|s| Decimal::from_str(s).ok())
}

fn event_duration(data: &Value) -> Option<Decimal> {
    data.get("event_duration")
        .and_then(decimal_from_json)
        .or_else(|| {
            data.get("properties")
                .and_then(|v| v.get("$event_duration"))
                .and_then(decimal_from_json)
        })
}

/// 写入单条 track_events 记录。
pub async fn save_track_event(
    db: &DatabaseConnection,
    project: &models::project::Model,
    data: &Value,
    ctx: &Value,
    session_id: Option<String>,
) -> AppResult<()> {
    let distinct_id = data_str(data, "distinct_id")
        .or_else(|| ctx_str(ctx, "distinct_id"))
        .ok_or_else(|| AppError::BadRequest("missing distinct_id".into()))?;

    let event =
        data_str(data, "event").ok_or_else(|| AppError::BadRequest("missing event".into()))?;

    let is_login_id = data
        .get("is_login_id")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let anonymous_id = data_str(data, "anonymous_id");
    let user_id = if is_login_id {
        Some(distinct_id.clone())
    } else {
        None
    };

    let event_type = if event.starts_with('$') {
        "auto"
    } else {
        "custom"
    };

    let active = models::track_event::ActiveModel {
        id: sea_orm::NotSet,
        project_id: Set(project.id),
        app_id: Set(project.app_id.clone()),
        distinct_id: Set(distinct_id.clone()),
        anonymous_id: Set(anonymous_id.clone()),
        user_id: Set(user_id),
        is_login_id: Set(is_login_id),
        event: Set(event),
        event_type: Set(event_type.into()),
        properties: Set(data.get("properties").cloned()),
        super_properties: Set(data.get("super_properties").cloned()),
        session_id: Set(session_id),
        event_duration: Set(event_duration(data)),
        page_url: Set(ctx_str(ctx, "url")),
        page_title: Set(ctx_str(ctx, "title")),
        referrer: Set(ctx_str(ctx, "referrer")),
        viewport: Set(ctx_str(ctx, "viewport")),
        screen_resolution: Set(ctx_str(ctx, "screen_resolution")),
        user_agent: Set(ctx_str(ctx, "user_agent")),
        browser: Set(ctx_str(ctx, "browser")),
        browser_version: Set(ctx_str(ctx, "browser_version")),
        os: Set(ctx_str(ctx, "os")),
        os_version: Set(ctx_str(ctx, "os_version")),
        device_type: Set(ctx_str(ctx, "device_type")),
        language: Set(ctx_str(ctx, "language")),
        timezone: Set(ctx_str(ctx, "timezone")),
        sdk_version: Set(ctx_str(ctx, "sdk_version")),
        release: Set(ctx_str(ctx, "release")),
        environment: Set(ctx_str(ctx, "environment")),
        client_time: Set(parse_client_time(data)),
        created_at: Set(now_fixed()),
    };
    active.insert(db).await?;

    // 顺手更新 profile 的 last_visit_at / total_events
    upsert_profile_touch(db, project.id, &distinct_id, anonymous_id).await?;
    Ok(())
}

/// profile 操作（set / set_once / append / unset）。
pub async fn save_profile(
    db: &DatabaseConnection,
    project: &models::project::Model,
    data: &Value,
) -> AppResult<()> {
    let distinct_id = data_str(data, "distinct_id")
        .ok_or_else(|| AppError::BadRequest("missing distinct_id".into()))?;
    let operation = data_str(data, "operation").unwrap_or_else(|| "set".into());
    let is_login_id = data
        .get("is_login_id")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let props = data.get("properties").cloned().unwrap_or(json!({}));

    let existing = models::TrackUserProfile::find()
        .filter(models::track_user_profile::Column::ProjectId.eq(project.id))
        .filter(models::track_user_profile::Column::DistinctId.eq(distinct_id.clone()))
        .one(db)
        .await?;

    let merged = match &existing {
        Some(row) => merge_profile(&row.properties, &props, &operation),
        None => {
            if operation == "unset" {
                json!({})
            } else {
                props.clone()
            }
        }
    };

    match existing {
        Some(row) => {
            let mut am: models::track_user_profile::ActiveModel = row.into();
            am.properties = Set(merged);
            if is_login_id {
                am.user_id = Set(Some(distinct_id.clone()));
            }
            am.last_visit_at = Set(Some(now_fixed()));
            am.updated_at = Set(now_fixed());
            am.update(db).await?;
        }
        None => {
            let active = models::track_user_profile::ActiveModel {
                id: sea_orm::NotSet,
                project_id: Set(project.id),
                distinct_id: Set(distinct_id.clone()),
                anonymous_id: Set(if is_login_id {
                    None
                } else {
                    Some(distinct_id.clone())
                }),
                user_id: Set(if is_login_id {
                    Some(distinct_id.clone())
                } else {
                    None
                }),
                name: Set(None),
                email: Set(None),
                phone: Set(None),
                properties: Set(merged),
                first_visit_at: Set(Some(now_fixed())),
                last_visit_at: Set(Some(now_fixed())),
                total_events: Set(0),
                total_sessions: Set(0),
                created_at: Set(now_fixed()),
                updated_at: Set(now_fixed()),
            };
            active.insert(db).await?;
        }
    }
    Ok(())
}

fn merge_profile(existing: &Value, incoming: &Value, op: &str) -> Value {
    let mut base = match existing {
        Value::Object(_) => existing.clone(),
        _ => json!({}),
    };
    let incoming_obj = match incoming {
        Value::Object(m) => m.clone(),
        _ => return base,
    };
    let base_obj = base.as_object_mut().unwrap();

    for (k, v) in incoming_obj {
        match op {
            "set" => {
                base_obj.insert(k, v);
            }
            "set_once" => {
                base_obj.entry(k).or_insert(v);
            }
            "append" => {
                let entry = base_obj.entry(k).or_insert_with(|| json!([]));
                if let Some(arr) = entry.as_array_mut() {
                    if let Value::Array(mut items) = v {
                        arr.append(&mut items);
                    } else {
                        arr.push(v);
                    }
                }
            }
            "unset" => {
                base_obj.remove(&k);
            }
            _ => {}
        }
    }
    base
}

/// 写入事件的同时更新 profile 的 last_visit_at / total_events。
async fn upsert_profile_touch(
    db: &DatabaseConnection,
    project_id: i32,
    distinct_id: &str,
    anonymous_id: Option<String>,
) -> AppResult<()> {
    let existing = models::TrackUserProfile::find()
        .filter(models::track_user_profile::Column::ProjectId.eq(project_id))
        .filter(models::track_user_profile::Column::DistinctId.eq(distinct_id))
        .one(db)
        .await?;

    match existing {
        Some(row) => {
            let total = row.total_events + 1;
            let mut am: models::track_user_profile::ActiveModel = row.into();
            am.total_events = Set(total);
            am.last_visit_at = Set(Some(now_fixed()));
            am.updated_at = Set(now_fixed());
            am.update(db).await?;
        }
        None => {
            let active = models::track_user_profile::ActiveModel {
                id: sea_orm::NotSet,
                project_id: Set(project_id),
                distinct_id: Set(distinct_id.to_string()),
                anonymous_id: Set(anonymous_id),
                user_id: Set(None),
                name: Set(None),
                email: Set(None),
                phone: Set(None),
                properties: Set(json!({})),
                first_visit_at: Set(Some(now_fixed())),
                last_visit_at: Set(Some(now_fixed())),
                total_events: Set(1),
                total_sessions: Set(0),
                created_at: Set(now_fixed()),
                updated_at: Set(now_fixed()),
            };
            active.insert(db).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_profile_set() {
        let existing = json!({"name": "Alice", "age": 25});
        let incoming = json!({"age": 30, "city": "Beijing"});
        let result = merge_profile(&existing, &incoming, "set");
        assert_eq!(result["name"], "Alice");
        assert_eq!(result["age"], 30);
        assert_eq!(result["city"], "Beijing");
    }

    #[test]
    fn test_merge_profile_set_once() {
        let existing = json!({"name": "Alice", "age": 25});
        let incoming = json!({"age": 30, "city": "Beijing"});
        let result = merge_profile(&existing, &incoming, "set_once");
        assert_eq!(result["name"], "Alice");
        assert_eq!(result["age"], 25);
        assert_eq!(result["city"], "Beijing");
    }

    #[test]
    fn test_merge_profile_unset() {
        let existing = json!({"name": "Alice", "age": 25});
        let incoming = json!({"age": null});
        let result = merge_profile(&existing, &incoming, "unset");
        assert_eq!(result["name"], "Alice");
        assert!(result.get("age").is_none());
    }

    #[test]
    fn test_merge_profile_append() {
        let existing = json!({"tags": ["a"]});
        let incoming = json!({"tags": ["b", "c"]});
        let result = merge_profile(&existing, &incoming, "append");
        assert_eq!(result["tags"], json!(["a", "b", "c"]));
    }

    #[test]
    fn test_merge_profile_unknown_op() {
        let existing = json!({"name": "Alice"});
        let incoming = json!({"name": "Bob"});
        let result = merge_profile(&existing, &incoming, "unknown");
        assert_eq!(result["name"], "Alice");
    }

    #[test]
    fn test_merge_profile_empty_existing() {
        let existing = json!({});
        let incoming = json!({"name": "Alice"});
        let result = merge_profile(&existing, &incoming, "set");
        assert_eq!(result["name"], "Alice");
    }

    #[test]
    fn test_parse_client_time_valid() {
        let data = json!({"client_time": 1705312200000i64});
        assert!(parse_client_time(&data).is_some());
    }

    #[test]
    fn test_parse_client_time_missing() {
        let data = json!({});
        assert!(parse_client_time(&data).is_none());
    }

    #[test]
    fn test_decimal_from_json_float() {
        assert!(decimal_from_json(&json!(3.15)).is_some());
    }

    #[test]
    fn test_decimal_from_json_string() {
        assert!(decimal_from_json(&json!("2.718")).is_some());
    }

    #[test]
    fn test_decimal_from_json_invalid() {
        assert!(decimal_from_json(&json!("not_a_number")).is_none());
    }

    #[test]
    fn test_event_duration_from_data() {
        assert!(event_duration(&json!({"event_duration": 1.5})).is_some());
    }

    #[test]
    fn test_event_duration_from_properties() {
        assert!(event_duration(&json!({"properties": {"$event_duration": 2.0}})).is_some());
    }

    #[test]
    fn test_event_duration_missing() {
        assert!(event_duration(&json!({})).is_none());
    }
}
