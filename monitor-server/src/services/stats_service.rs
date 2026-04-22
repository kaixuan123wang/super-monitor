//! 埋点事件按小时预聚合服务（Phase 3）。
//!
//! 每分钟执行一次：
//!   1. 扫描最近 2 小时内的 track_events
//!   2. 按 (project_id, event, hour) 聚合 total_count / unique_users
//!   3. Upsert 到 track_event_stats_hourly
//!
//! 在 main.rs 中以 tokio::spawn 独立任务运行。

use chrono::{DateTime, Duration, FixedOffset, TimeZone, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use tracing::{info, warn};

use crate::models;

/// 启动后台预聚合循环（每分钟跑一次）。
pub async fn start_aggregation_loop(db: DatabaseConnection) {
    loop {
        if let Err(e) = run_aggregation(&db).await {
            warn!(error = %e, "stats aggregation failed");
        }
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}

/// 执行一次预聚合。
async fn run_aggregation(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    let since = (Utc::now() - Duration::hours(2))
        .with_timezone(&FixedOffset::east_opt(0).unwrap());

    let rows = models::TrackEvent::find()
        .filter(models::track_event::Column::CreatedAt.gte(since))
        .all(db)
        .await?;

    if rows.is_empty() {
        return Ok(());
    }

    // 按 (project_id, event, hour) 聚合
    // key: (project_id, event_name, hour_ts)
    let mut buckets: HashMap<(i32, String, i64), (u32, HashSet<String>)> = HashMap::new();

    for row in &rows {
        let hour_ts = truncate_to_hour(row.created_at);
        let key = (row.project_id, row.event.clone(), hour_ts);
        let entry = buckets.entry(key).or_default();
        entry.0 += 1;
        entry.1.insert(row.distinct_id.clone());
    }

    let mut upserted = 0u32;
    for ((project_id, event, hour_ts), (total, users)) in buckets {
        let hour = Utc
            .timestamp_opt(hour_ts, 0)
            .single()
            .unwrap_or_else(Utc::now)
            .with_timezone(&FixedOffset::east_opt(0).unwrap());

        // 查询是否已存在
        let existing = models::TrackEventStats::find()
            .filter(models::track_event_stats::Column::ProjectId.eq(project_id))
            .filter(models::track_event_stats::Column::Event.eq(event.clone()))
            .filter(models::track_event_stats::Column::Hour.eq(hour))
            .one(db)
            .await?;

        match existing {
            Some(row) => {
                // 累加（保守：取最新聚合值，不再加旧值）
                let new_total = total.max(row.total_count as u32);
                let new_users = (users.len() as i32).max(row.unique_users);
                let mut am: models::track_event_stats::ActiveModel = row.into();
                am.total_count = Set(new_total as i32);
                am.unique_users = Set(new_users);
                am.update(db).await?;
            }
            None => {
                let am = models::track_event_stats::ActiveModel {
                    id: sea_orm::NotSet,
                    project_id: Set(project_id),
                    event: Set(event),
                    hour: Set(hour),
                    total_count: Set(total as i32),
                    unique_users: Set(users.len() as i32),
                    properties_summary: Set(Some(json!({}))),
                };
                am.insert(db).await?;
            }
        }
        upserted += 1;
    }

    if upserted > 0 {
        info!(upserted, "track_event_stats_hourly aggregation done");
    }
    Ok(())
}

/// 将时间戳截断到整小时（UTC 秒级时间戳）。
fn truncate_to_hour(dt: DateTime<FixedOffset>) -> i64 {
    let ts = dt.timestamp();
    ts - (ts % 3600)
}
