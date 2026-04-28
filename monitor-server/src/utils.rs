//! 共享工具函数。

use chrono::{FixedOffset, Utc};
use sea_orm::{DatabaseConnection, EntityTrait};

use crate::error::{AppError, AppResult};
use crate::middleware::auth::CurrentUser;
use crate::models;
use crate::router::AppState;

/// 获取数据库连接，如果未连接则返回错误。
pub fn get_db(state: &AppState) -> AppResult<&DatabaseConnection> {
    state
        .db
        .as_ref()
        .ok_or_else(|| AppError::Internal("database not connected".into()))
}

/// 获取当前时间（UTC 固定时区）。
pub fn now_fixed() -> chrono::DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap())
}

/// 默认页码（从 1 开始）。
pub fn default_page() -> u64 {
    1
}

/// 默认每页条数。
pub fn default_page_size() -> u64 {
    20
}

/// 分页查询参数。
#[derive(Debug, serde::Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
}

impl PaginationParams {
    /// 返回经过上限（100）约束后的 page_size，防止超大查询。
    pub fn safe_page_size(&self) -> u64 {
        self.page_size.clamp(1, 100)
    }
}

/// 验证 SSE 短生命周期 token（从 Redis 中原子读取并删除，一次性使用）。
pub async fn validate_sse_token(state: &AppState, token: &str) -> AppResult<CurrentUser> {
    // Redis 是 SSE token 验证的必要组件
    let redis = state
        .redis
        .as_ref()
        .ok_or_else(|| AppError::Internal("SSE authentication service unavailable".into()))?;

    let key = format!("sse_token:{}", token);
    let mut conn = redis.clone();
    let script = r#"
        local value = redis.call('GET', KEYS[1])
        if value then
            redis.call('DEL', KEYS[1])
        end
        return value
    "#;
    let result: Option<String> = redis::cmd("EVAL")
        .arg(script)
        .arg(1)
        .arg(&key)
        .query_async(&mut conn)
        .await
        .unwrap_or(None);

    let user_id = result
        .ok_or(AppError::Unauthorized)?
        .parse::<i32>()
        .map_err(|_| AppError::Unauthorized)?;

    let db = get_db(state)?;
    let user = models::User::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or(AppError::Unauthorized)?;

    Ok(CurrentUser { id: user.id, username: user.username, role: user.role })
}

/// 标准分页响应。
pub fn paginated_response<T: serde::Serialize>(
    list: Vec<T>,
    total: u64,
    page: u64,
    page_size: u64,
) -> serde_json::Value {
    serde_json::json!({
        "code": 0,
        "message": "ok",
        "data": { "list": list, "total": total },
        "pagination": {
            "page": page,
            "page_size": page_size,
            "total": total,
            "total_pages": (total as f64 / page_size as f64).ceil() as u64
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_page() {
        assert_eq!(default_page(), 1);
    }

    #[test]
    fn test_default_page_size() {
        assert_eq!(default_page_size(), 20);
    }

    #[test]
    fn test_now_fixed_is_utc() {
        let now = now_fixed();
        assert_eq!(now.offset().local_minus_utc(), 0);
    }

    #[test]
    fn test_pagination_params_defaults() {
        let params: PaginationParams = serde_json::from_str("{}").unwrap();
        assert_eq!(params.page, 1);
        assert_eq!(params.page_size, 20);
    }

    #[test]
    fn test_pagination_params_custom() {
        let params: PaginationParams =
            serde_json::from_str(r#"{"page": 3, "page_size": 50}"#).unwrap();
        assert_eq!(params.page, 3);
        assert_eq!(params.page_size, 50);
    }

    #[test]
    fn test_paginated_response_structure() {
        let resp = paginated_response(vec!["a", "b"], 10, 1, 2);
        assert_eq!(resp["code"], 0);
        assert_eq!(resp["message"], "ok");
        assert_eq!(resp["data"]["total"], 10);
        assert_eq!(resp["data"]["list"].as_array().unwrap().len(), 2);
        assert_eq!(resp["pagination"]["page"], 1);
        assert_eq!(resp["pagination"]["page_size"], 2);
        assert_eq!(resp["pagination"]["total"], 10);
        assert_eq!(resp["pagination"]["total_pages"], 5);
    }

    #[test]
    fn test_paginated_response_empty() {
        let resp = paginated_response(Vec::<String>::new(), 0, 1, 20);
        assert_eq!(resp["data"]["total"], 0);
        assert_eq!(resp["pagination"]["total_pages"], 0);
    }

    #[test]
    fn test_paginated_response_rounding() {
        let resp = paginated_response(Vec::<String>::new(), 21, 1, 20);
        assert_eq!(resp["pagination"]["total_pages"], 2);
    }

    #[test]
    fn test_get_db_none_returns_error() {
        use crate::config::Config;
        use tokio::sync::broadcast;
        let (tx, _) = broadcast::channel(1);
        let state = crate::router::AppState {
            config: Config {
                database_url: String::new(),
                redis_url: String::new(),
                jwt_secret: String::new(),
                ai_api_key: String::new(),
                ai_api_base: "https://api.deepseek.com/v1".into(),
                ai_model: "deepseek-chat".into(),
                ai_enabled: true,
                sourcemap_dir: "./data/sourcemaps".into(),
                server_port: 8080,
                sse_port: 8081,
                cors_origins: String::new(),
            },
            db: None,
            alert_tx: tx,
            redis: None,
        };
        let result = get_db(&state);
        assert!(result.is_err());
    }
}
