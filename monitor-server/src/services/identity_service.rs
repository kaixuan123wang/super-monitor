//! 用户 ID 关联合并服务（匿名 ID ↔ 登录 ID）。
//!
//! 写入 `track_id_mapping`，并把匿名阶段事件 / Profile 合并到登录 ID。
//! 复杂的跨设备多 ID 图谱仍留到后续阶段。

use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::{
    sea_query::Expr, ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    Set,
};
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::models;

fn now_fixed() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap())
}

pub async fn save_id_mapping(
    db: &DatabaseConnection,
    project_id: i32,
    data: &Value,
) -> AppResult<()> {
    let login_id = data
        .get("distinct_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("missing distinct_id".into()))?
        .to_string();
    let anonymous_id = data
        .get("original_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("missing original_id".into()))?
        .to_string();

    if login_id == anonymous_id {
        return Ok(());
    }

    let existing = models::TrackIdMapping::find()
        .filter(models::track_id_mapping::Column::ProjectId.eq(project_id))
        .filter(models::track_id_mapping::Column::AnonymousId.eq(anonymous_id.clone()))
        .one(db)
        .await?;
    if existing.is_none() {
        let active = models::track_id_mapping::ActiveModel {
            id: sea_orm::NotSet,
            project_id: Set(project_id),
            anonymous_id: Set(anonymous_id.clone()),
            login_id: Set(login_id.clone()),
            merged_at: Set(now_fixed()),
        };
        active.insert(db).await?;
    }

    merge_profile(db, project_id, &anonymous_id, &login_id).await?;
    merge_events(db, project_id, &anonymous_id, &login_id).await?;
    Ok(())
}

async fn merge_events(
    db: &DatabaseConnection,
    project_id: i32,
    anonymous_id: &str,
    login_id: &str,
) -> AppResult<()> {
    models::TrackEvent::update_many()
        .col_expr(models::track_event::Column::DistinctId, Expr::value(login_id.to_string()))
        .col_expr(models::track_event::Column::UserId, Expr::value(login_id.to_string()))
        .col_expr(models::track_event::Column::IsLoginId, Expr::value(true))
        .filter(models::track_event::Column::ProjectId.eq(project_id))
        .filter(models::track_event::Column::DistinctId.eq(anonymous_id))
        .exec(db)
        .await?;
    Ok(())
}

async fn merge_profile(
    db: &DatabaseConnection,
    project_id: i32,
    anonymous_id: &str,
    login_id: &str,
) -> AppResult<()> {
    let anon = models::TrackUserProfile::find()
        .filter(models::track_user_profile::Column::ProjectId.eq(project_id))
        .filter(models::track_user_profile::Column::DistinctId.eq(anonymous_id))
        .one(db)
        .await?;
    let login = models::TrackUserProfile::find()
        .filter(models::track_user_profile::Column::ProjectId.eq(project_id))
        .filter(models::track_user_profile::Column::DistinctId.eq(login_id))
        .one(db)
        .await?;

    match (anon, login) {
        (Some(anon), Some(login)) => {
            let mut am: models::track_user_profile::ActiveModel = login.clone().into();
            am.anonymous_id = Set(login.anonymous_id.or(Some(anonymous_id.to_string())));
            am.user_id = Set(Some(login_id.to_string()));
            am.properties = Set(merge_properties(&anon.properties, &login.properties));
            am.first_visit_at = Set(min_time(anon.first_visit_at, login.first_visit_at));
            am.last_visit_at = Set(max_time(anon.last_visit_at, login.last_visit_at));
            am.total_events = Set(anon.total_events + login.total_events);
            am.total_sessions = Set(anon.total_sessions + login.total_sessions);
            am.updated_at = Set(now_fixed());
            am.update(db).await?;
            models::TrackUserProfile::delete_by_id(anon.id)
                .exec(db)
                .await?;
        }
        (Some(anon), None) => {
            let mut am: models::track_user_profile::ActiveModel = anon.into();
            am.distinct_id = Set(login_id.to_string());
            am.anonymous_id = Set(Some(anonymous_id.to_string()));
            am.user_id = Set(Some(login_id.to_string()));
            am.updated_at = Set(now_fixed());
            am.update(db).await?;
        }
        (None, Some(login)) => {
            let mut am: models::track_user_profile::ActiveModel = login.into();
            am.anonymous_id = Set(Some(anonymous_id.to_string()));
            am.user_id = Set(Some(login_id.to_string()));
            am.updated_at = Set(now_fixed());
            am.update(db).await?;
        }
        (None, None) => {
            let active = models::track_user_profile::ActiveModel {
                id: sea_orm::NotSet,
                project_id: Set(project_id),
                distinct_id: Set(login_id.to_string()),
                anonymous_id: Set(Some(anonymous_id.to_string())),
                user_id: Set(Some(login_id.to_string())),
                name: Set(None),
                email: Set(None),
                phone: Set(None),
                properties: Set(json!({})),
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

fn merge_properties(anonymous: &Value, login: &Value) -> Value {
    let mut merged = match anonymous {
        Value::Object(_) => anonymous.clone(),
        _ => json!({}),
    };
    if let (Some(base), Some(login_obj)) = (merged.as_object_mut(), login.as_object()) {
        for (key, value) in login_obj {
            base.insert(key.clone(), value.clone());
        }
    }
    merged
}

fn min_time(
    a: Option<DateTime<FixedOffset>>,
    b: Option<DateTime<FixedOffset>>,
) -> Option<DateTime<FixedOffset>> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn max_time(
    a: Option<DateTime<FixedOffset>>,
    b: Option<DateTime<FixedOffset>>,
) -> Option<DateTime<FixedOffset>> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_merge_properties_both_objects() {
        let anon = json!({"a": 1, "b": 2});
        let login = json!({"b": 3, "c": 4});
        let result = merge_properties(&anon, &login);
        assert_eq!(result["a"], 1);
        assert_eq!(result["b"], 3); // login overwrites
        assert_eq!(result["c"], 4);
    }

    #[test]
    fn test_merge_properties_anon_not_object() {
        let anon = json!("not object");
        let login = json!({"a": 1});
        let result = merge_properties(&anon, &login);
        assert_eq!(result["a"], 1);
    }

    #[test]
    fn test_merge_properties_login_not_object() {
        let anon = json!({"a": 1});
        let login = json!("not object");
        let result = merge_properties(&anon, &login);
        assert_eq!(result["a"], 1);
    }

    #[test]
    fn test_merge_properties_empty() {
        let result = merge_properties(&json!({}), &json!({}));
        assert_eq!(result, json!({}));
    }

    #[test]
    fn test_min_time_both_some() {
        let a = FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
            .unwrap();
        let b = FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2024, 1, 2, 0, 0, 0)
            .unwrap();
        assert_eq!(min_time(Some(a), Some(b)), Some(a));
    }

    #[test]
    fn test_min_time_first_none() {
        let b = FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
            .unwrap();
        assert_eq!(min_time(None, Some(b)), Some(b));
    }

    #[test]
    fn test_min_time_second_none() {
        let a = FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
            .unwrap();
        assert_eq!(min_time(Some(a), None), Some(a));
    }

    #[test]
    fn test_min_time_both_none() {
        assert_eq!(min_time(None, None), None);
    }

    #[test]
    fn test_max_time_both_some() {
        let a = FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
            .unwrap();
        let b = FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2024, 1, 2, 0, 0, 0)
            .unwrap();
        assert_eq!(max_time(Some(a), Some(b)), Some(b));
    }

    #[test]
    fn test_max_time_first_none() {
        let b = FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
            .unwrap();
        assert_eq!(max_time(None, Some(b)), Some(b));
    }

    #[test]
    fn test_max_time_second_none() {
        let a = FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
            .unwrap();
        assert_eq!(max_time(Some(a), None), Some(a));
    }

    #[test]
    fn test_max_time_both_none() {
        assert_eq!(max_time(None, None), None);
    }

    #[test]
    fn test_now_fixed_is_utc() {
        let now = now_fixed();
        assert_eq!(now.offset().local_minus_utc(), 0);
    }
}
