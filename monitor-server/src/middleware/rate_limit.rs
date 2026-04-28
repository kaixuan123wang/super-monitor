//! 基于 Redis 的限流中间件（Token Bucket）。

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{AppError, AppResult};
use crate::router::AppState;

/// 限流配置。
pub struct RateLimitConfig {
    /// 每秒允许的最大请求数
    pub requests_per_second: u32,
    /// 突发容量（桶大小）
    pub burst_size: u32,
    /// 限流 key 前缀
    pub key_prefix: String,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self { requests_per_second: 10, burst_size: 20, key_prefix: "rate_limit".into() }
    }
}

/// 基于 Redis 的 Token Bucket 限流中间件。
///
/// 使用 AppState 中的共享 Redis 连接，避免每个请求创建新连接。
/// 若 Redis 不可用则放行（降级）。
pub async fn rate_limit(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> AppResult<Response> {
    let cfg = RateLimitConfig::default();
    let key = format!("{}:{}", cfg.key_prefix, client_ip(&req));

    // 使用共享的 Redis 连接
    if let Some(ref conn) = state.redis {
        let mut conn = conn.clone();
        if let Ok(allowed) =
            check_redis_limit(&mut conn, &key, cfg.requests_per_second, cfg.burst_size).await
        {
            if !allowed {
                return Err(AppError::TooManyRequests("请求过于频繁，请稍后再试".into()));
            }
        }
    }

    Ok(next.run(req).await)
}

/// 从请求中提取客户端 IP。
fn client_ip(req: &Request) -> String {
    req.headers()
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            req.headers()
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.split(',').next_back())
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_else(|| "unknown".into())
}

/// 使用 Redis 执行 Token Bucket 限流检查。
async fn check_redis_limit(
    conn: &mut redis::aio::MultiplexedConnection,
    key: &str,
    rate: u32,
    burst: u32,
) -> Result<bool, Box<dyn std::error::Error>> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs_f64();

    // Lua 脚本：原子性更新 token bucket
    let script = r#"
        local key = KEYS[1]
        local rate = tonumber(ARGV[1])
        local burst = tonumber(ARGV[2])
        local now = tonumber(ARGV[3])
        local interval = 1.0 / rate

        local tokens = redis.call('hget', key, 'tokens')
        local last = redis.call('hget', key, 'last')

        if tokens == false then
            tokens = burst
            last = now
        else
            tokens = tonumber(tokens)
            last = tonumber(last)
            local elapsed = now - last
            tokens = math.min(burst, tokens + elapsed / interval)
        end

        if tokens >= 1 then
            tokens = tokens - 1
            redis.call('hmset', key, 'tokens', tokens, 'last', now)
            redis.call('expire', key, 60)
            return 1
        else
            redis.call('hmset', key, 'tokens', tokens, 'last', now)
            redis.call('expire', key, 60)
            return 0
        end
    "#;

    let result: i32 = redis::cmd("EVAL")
        .arg(script)
        .arg(1)
        .arg(key)
        .arg(rate)
        .arg(burst)
        .arg(now)
        .query_async(conn)
        .await?;

    Ok(result == 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config_defaults() {
        let cfg = RateLimitConfig::default();
        assert_eq!(cfg.requests_per_second, 10);
        assert_eq!(cfg.burst_size, 20);
        assert_eq!(cfg.key_prefix, "rate_limit");
    }

    #[test]
    fn test_client_ip_from_x_forwarded_for() {
        let req = Request::builder()
            .header("x-forwarded-for", "1.2.3.4, 5.6.7.8")
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(client_ip(&req), "5.6.7.8");
    }

    #[test]
    fn test_client_ip_from_x_real_ip() {
        let req = Request::builder()
            .header("x-real-ip", "10.0.0.1")
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(client_ip(&req), "10.0.0.1");
    }

    #[test]
    fn test_client_ip_forwarded_for_takes_priority() {
        let req = Request::builder()
            .header("x-forwarded-for", "1.1.1.1")
            .header("x-real-ip", "2.2.2.2")
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(client_ip(&req), "2.2.2.2");
    }

    #[test]
    fn test_client_ip_unknown_fallback() {
        let req = Request::builder().body(axum::body::Body::empty()).unwrap();
        assert_eq!(client_ip(&req), "unknown");
    }

    #[test]
    fn test_client_ip_single_forwarded() {
        let req = Request::builder()
            .header("x-forwarded-for", "192.168.1.1")
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(client_ip(&req), "192.168.1.1");
    }
}
