//! 告警规则匹配、去重、广播推送服务。

use chrono::{Duration, FixedOffset, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::sync::broadcast;
use tracing::{info, warn};

use crate::models;

/// 告警事件，通过 broadcast channel 传播给 SSE handler。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlertEvent {
    pub rule_id: i32,
    pub rule_name: String,
    pub project_id: i32,
    pub alert_type: String,
    pub severity: String,
    pub content: String,
    pub error_count: Option<i32>,
    pub sample_errors: Option<Value>,
    pub timestamp: chrono::DateTime<FixedOffset>,
}

fn now_fixed() -> chrono::DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap())
}

/// 后台循环：每 60s 检查一次所有启用的告警规则。
pub async fn start_alert_loop(db: DatabaseConnection, tx: broadcast::Sender<AlertEvent>) {
    loop {
        if let Err(e) = run_alert_check(&db, &tx).await {
            warn!(error = %e, "alert check failed");
        }
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}

async fn run_alert_check(
    db: &DatabaseConnection,
    tx: &broadcast::Sender<AlertEvent>,
) -> Result<(), sea_orm::DbErr> {
    let rules = models::AlertRule::find()
        .filter(models::alert_rule::Column::IsEnabled.eq(true))
        .all(db)
        .await?;

    for rule in rules {
        if let Err(e) = check_rule(db, &rule, tx).await {
            warn!(rule_id = rule.id, error = %e, "rule check error");
        }
    }
    Ok(())
}

async fn check_rule(
    db: &DatabaseConnection,
    rule: &models::alert_rule::Model,
    tx: &broadcast::Sender<AlertEvent>,
) -> Result<(), sea_orm::DbErr> {
    let window = Duration::minutes(rule.interval_minutes as i64);
    let now = Utc::now();
    let since = (now - window).with_timezone(&FixedOffset::east_opt(0).unwrap());

    match rule.rule_type.as_str() {
        "error_spike" => {
            let threshold = rule.threshold.unwrap_or(10);
            let errors = models::JsError::find()
                .filter(models::js_error::Column::ProjectId.eq(rule.project_id))
                .filter(models::js_error::Column::CreatedAt.gte(since))
                .all(db)
                .await?;

            let count = errors.len() as i32;
            if count >= threshold {
                let sample: Vec<Value> = errors.iter().take(3).map(|e| {
                    json!({ "id": e.id, "message": truncate_chars(&e.message, 120), "url": e.url, "fingerprint": e.fingerprint })
                }).collect();
                let content = format!(
                    "{}分钟内检测到 {} 个错误，超过阈值 {}",
                    rule.interval_minutes, count, threshold
                );
                let dedup_key = format!("error_spike:{}:{}", rule.id, rule.interval_minutes);
                fire_alert(
                    db,
                    tx,
                    rule,
                    "error_spike",
                    "warning",
                    &content,
                    Some(count),
                    Some(json!(sample)),
                    &dedup_key,
                )
                .await?;
            }
        }
        "failure_rate" => {
            let threshold = rule.threshold.unwrap_or(50).clamp(1, 100);
            let rows = models::NetworkError::find()
                .filter(models::network_error::Column::ProjectId.eq(rule.project_id))
                .filter(models::network_error::Column::CreatedAt.gte(since))
                .all(db)
                .await?;

            let total = rows.len() as i32;
            if total > 0 {
                let failed = rows
                    .iter()
                    .filter(|r| r.status.map_or(true, |s| s >= 400) || r.error_type.is_some())
                    .count() as i32;
                let rate = ((failed as f64 / total as f64) * 100.0).round() as i32;
                if rate >= threshold {
                    let sample: Vec<Value> = rows.iter().take(3).map(|e| {
                        json!({ "id": e.id, "url": e.url, "method": e.method, "status": e.status, "error_type": e.error_type })
                    }).collect();
                    let content = format!(
                        "{}分钟内接口失败率 {}%（{}/{}），超过阈值 {}%",
                        rule.interval_minutes, rate, failed, total, threshold
                    );
                    let dedup_key = format!("failure_rate:{}:{}", rule.id, rule.interval_minutes);
                    fire_alert(
                        db,
                        tx,
                        rule,
                        "failure_rate",
                        "warning",
                        &content,
                        Some(failed),
                        Some(json!(sample)),
                        &dedup_key,
                    )
                    .await?;
                }
            }
        }
        "error_trend" => {
            let threshold = rule.threshold.unwrap_or(50).max(1);
            let previous_since =
                (now - window - window).with_timezone(&FixedOffset::east_opt(0).unwrap());
            let current_count = models::JsError::find()
                .filter(models::js_error::Column::ProjectId.eq(rule.project_id))
                .filter(models::js_error::Column::CreatedAt.gte(since))
                .count(db)
                .await? as i32;
            let previous_count = models::JsError::find()
                .filter(models::js_error::Column::ProjectId.eq(rule.project_id))
                .filter(models::js_error::Column::CreatedAt.gte(previous_since))
                .filter(models::js_error::Column::CreatedAt.lt(since))
                .count(db)
                .await? as i32;

            let growth = if previous_count == 0 {
                if current_count > 0 {
                    100
                } else {
                    0
                }
            } else {
                (((current_count - previous_count) as f64 / previous_count as f64) * 100.0).round()
                    as i32
            };

            if current_count > 0 && growth >= threshold {
                let content = format!(
                    "错误数较上一窗口增长 {}%（当前 {}，上一窗口 {}），超过阈值 {}%",
                    growth, current_count, previous_count, threshold
                );
                let dedup_key = format!("error_trend:{}:{}", rule.id, rule.interval_minutes);
                fire_alert(
                    db,
                    tx,
                    rule,
                    "error_trend",
                    "warning",
                    &content,
                    Some(current_count),
                    None,
                    &dedup_key,
                )
                .await?;
            }
        }
        "new_error" => {
            let recent_errors = models::JsError::find()
                .filter(models::js_error::Column::ProjectId.eq(rule.project_id))
                .filter(models::js_error::Column::CreatedAt.gte(since))
                .order_by_desc(models::js_error::Column::CreatedAt)
                .all(db)
                .await?;

            let mut seen_fps: HashMap<String, &models::js_error::Model> = HashMap::new();
            for e in &recent_errors {
                if let Some(fp) = &e.fingerprint {
                    seen_fps.entry(fp.clone()).or_insert(e);
                }
            }

            for (fp, first) in seen_fps {
                let existed_before = models::JsError::find()
                    .filter(models::js_error::Column::ProjectId.eq(rule.project_id))
                    .filter(models::js_error::Column::Fingerprint.eq(fp.as_str()))
                    .filter(models::js_error::Column::CreatedAt.lt(since))
                    .one(db)
                    .await?
                    .is_some();
                if existed_before {
                    continue;
                }

                let content = format!(
                    "检测到新错误类型：{}（fingerprint: {}）",
                    truncate_chars(&first.message, 100),
                    fp
                );
                let sample = json!([{ "id": first.id, "message": first.message, "url": first.url, "fingerprint": fp }]);
                let dedup_key = format!("new_error:{}:{}", rule.id, fp);
                fire_alert(
                    db,
                    tx,
                    rule,
                    "new_error",
                    "info",
                    &content,
                    Some(1),
                    Some(sample),
                    &dedup_key,
                )
                .await?;
            }
        }
        "p0_error" => {
            let p0_errors = models::JsError::find()
                .filter(models::js_error::Column::ProjectId.eq(rule.project_id))
                .filter(models::js_error::Column::CreatedAt.gte(since))
                .filter(models::js_error::Column::Environment.eq("production"))
                .all(db)
                .await?;

            if !p0_errors.is_empty() {
                let e = &p0_errors[0];
                let content = format!("生产环境 P0 错误：{}", truncate_chars(&e.message, 120));
                let dedup_fp = e.fingerprint.as_deref().unwrap_or("unknown");
                let dedup_key = format!("p0_error:{}:{}", rule.id, dedup_fp);
                let sample = json!([{ "id": e.id, "message": e.message, "url": e.url, "fingerprint": e.fingerprint }]);
                fire_alert(
                    db,
                    tx,
                    rule,
                    "p0_error",
                    "critical",
                    &content,
                    Some(p0_errors.len() as i32),
                    Some(sample),
                    &dedup_key,
                )
                .await?;
            }
        }
        _ => {}
    }
    Ok(())
}

async fn fire_alert(
    db: &DatabaseConnection,
    tx: &broadcast::Sender<AlertEvent>,
    rule: &models::alert_rule::Model,
    alert_type: &str,
    severity: &str,
    content: &str,
    error_count: Option<i32>,
    sample_errors: Option<Value>,
    dedup_key: &str,
) -> Result<(), sea_orm::DbErr> {
    // 去重：10 分钟内同规则 + 同上下文不重复触发。
    let dedup_window =
        (Utc::now() - Duration::minutes(10)).with_timezone(&FixedOffset::east_opt(0).unwrap());
    let recent_logs = models::AlertLog::find()
        .filter(models::alert_log::Column::RuleId.eq(rule.id))
        .filter(models::alert_log::Column::AlertType.eq(alert_type))
        .filter(models::alert_log::Column::CreatedAt.gte(dedup_window))
        .all(db)
        .await?;

    let already_sent = recent_logs.iter().any(|log| {
        log.sample_errors
            .as_ref()
            .and_then(|v| v.get("dedup_key"))
            .and_then(|v| v.as_str())
            == Some(dedup_key)
    });
    if already_sent {
        return Ok(());
    }

    let escalation_window =
        (Utc::now() - Duration::minutes(30)).with_timezone(&FixedOffset::east_opt(0).unwrap());
    let recent_count = models::AlertLog::find()
        .filter(models::alert_log::Column::RuleId.eq(rule.id))
        .filter(models::alert_log::Column::CreatedAt.gte(escalation_window))
        .count(db)
        .await? as i32;
    let escalation_level = if recent_count >= 2 {
        Some(recent_count + 1)
    } else {
        None
    };
    let final_severity = if escalation_level.is_some() && severity != "critical" {
        "critical"
    } else {
        severity
    };
    let final_content = if let Some(level) = escalation_level {
        format!("{content}（30分钟内第 {level} 次触发，已升级）")
    } else {
        content.to_string()
    };
    let wrapped_samples = json!({
        "dedup_key": dedup_key,
        "items": sample_errors.unwrap_or_else(|| json!([])),
        "escalation_level": escalation_level,
    });
    let event = AlertEvent {
        rule_id: rule.id,
        rule_name: rule.name.clone(),
        project_id: rule.project_id,
        alert_type: alert_type.into(),
        severity: final_severity.into(),
        content: final_content.clone(),
        error_count,
        sample_errors: Some(wrapped_samples.clone()),
        timestamp: now_fixed(),
    };

    let mut status = "sent".to_string();
    if let Some(url) = rule.webhook_url.as_deref().filter(|s| !s.trim().is_empty()) {
        if let Err(e) = send_webhook(url, &event).await {
            warn!(rule_id = rule.id, error = %e, "webhook notification failed");
            status = "webhook_failed".into();
        }
    }
    if rule.email.as_deref().is_some_and(|s| !s.trim().is_empty()) {
        warn!(
            rule_id = rule.id,
            "email notification configured but SMTP delivery is not configured"
        );
        status = if status == "sent" {
            "email_pending".into()
        } else {
            "partial_failed".into()
        };
    }

    let log = models::alert_log::ActiveModel {
        id: sea_orm::NotSet,
        rule_id: Set(rule.id),
        project_id: Set(rule.project_id),
        alert_type: Set(alert_type.into()),
        severity: Set(final_severity.into()),
        content: Set(final_content),
        error_count: Set(error_count),
        sample_errors: Set(Some(wrapped_samples)),
        status: Set(status),
        created_at: Set(now_fixed()),
    };
    log.insert(db).await?;

    // broadcast 推送
    let _ = tx.send(event);
    info!(rule_id = rule.id, alert_type, "alert fired");
    Ok(())
}

async fn send_webhook(url: &str, event: &AlertEvent) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post(url)
        .json(&json!({ "type": "alert", "data": event }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(format!("HTTP {}", resp.status()))
    }
}

fn truncate_chars(s: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for (idx, ch) in s.chars().enumerate() {
        if idx >= max_chars {
            out.push('…');
            break;
        }
        out.push(ch);
    }
    out
}

/// 在接收到新错误时即时检查告警规则（由 sdk handler 调用）。
pub async fn check_on_new_error(
    db: &DatabaseConnection,
    tx: &broadcast::Sender<AlertEvent>,
    project_id: i32,
    _error: &models::js_error::Model,
) {
    let rules_result = models::AlertRule::find()
        .filter(models::alert_rule::Column::ProjectId.eq(project_id))
        .filter(models::alert_rule::Column::IsEnabled.eq(true))
        .all(db)
        .await;

    let rules = match rules_result {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "failed to load alert rules");
            return;
        }
    };

    for rule in &rules {
        if let Err(e) = check_rule(db, rule, tx).await {
            warn!(rule_id = rule.id, error = %e, "instant rule check error");
        }
    }
}
