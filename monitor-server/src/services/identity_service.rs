//! 用户 ID 关联合并服务（匿名 ID ↔ 登录 ID）。
//!
//! 当前实现：写入 track_id_mapping，重复 (project_id, anonymous_id) 忽略。
//! 复杂的跨设备合并留到后续阶段。

use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde_json::Value;

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
    if existing.is_some() {
        return Ok(());
    }

    let active = models::track_id_mapping::ActiveModel {
        id: sea_orm::NotSet,
        project_id: Set(project_id),
        anonymous_id: Set(anonymous_id),
        login_id: Set(login_id),
        merged_at: Set(now_fixed()),
    };
    active.insert(db).await?;
    Ok(())
}
