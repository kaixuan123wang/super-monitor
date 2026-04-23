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
        Self {
            requests_per_second: 10,
            burst_size: 20,
            key_prefix: "rate_limit".into(),
        }
    }
}

/// 基于 Redis 的 Token Bucket 限流中间件。
///
/// 使用 Redis 存储每个 key 的剩余 token 数和上次更新时间，
/// 支持分布式部署下的统一限流。
pub async fn rate_limit(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> AppResult<Response> {
    let cfg = RateLimitConfig::default();
    let key = format!("{}:{}", cfg.key_prefix, client_ip(&req));

    // 尝试从 Redis 检查限流
    if let Ok(allowed) = check_redis_limit(&state.config.redis_url,
        &key,
        cfg.requests_per_second,
        cfg.burst_size,
    ).await {
        if !allowed {
            return Err(AppError::TooManyRequests(
                "请求过于频繁，请稍后再试".into(),
            ));
        }
    }

    Ok(next.run(req).await)
}

/// 从请求中提取客户端 IP。
fn client_ip(req: &Request) -> String {
    req.headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            req.headers()
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "unknown".into())
}

/// 使用 Redis 执行 Token Bucket 限流检查。
/// 若 Redis 不可用则放行（降级）。
async fn check_redis_limit(
    redis_url: &str,
    key: &str,
    rate: u32,
    burst: u32,
) -> Result<bool, Box<dyn std::error::Error>> {
    let client = redis::Client::open(redis_url)?;
    let mut conn = client.get_multiplexed_async_connection().await?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs_f64();

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
        .query_async(&mut conn)
        .await?;

    Ok(result == 1)
}
