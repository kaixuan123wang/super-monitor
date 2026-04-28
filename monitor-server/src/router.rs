use axum::{
    extract::DefaultBodyLimit,
    http::header::HeaderName,
    middleware,
    routing::{delete, get, post, put},
    Json, Router,
};
use sea_orm::DatabaseConnection;
use serde_json::json;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::config::Config;
use crate::handlers;
use crate::middleware::{auth, rate_limit, security_headers};
use crate::services::alert_service::AlertEvent;

/// 共享应用状态（通过 Axum Extension / State 传递）。
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: Option<DatabaseConnection>,
    pub alert_tx: broadcast::Sender<AlertEvent>,
    pub redis: Option<redis::aio::MultiplexedConnection>,
}

/// 构建根 Router。
pub fn build_router(state: AppState) -> Router {
    // CORS 配置：从环境变量读取允许的来源
    let cors = if state.config.cors_origins.is_empty() {
        // 未配置时不允许任何跨域来源（安全默认值）
        CorsLayer::new()
            .allow_origin(tower_http::cors::AllowOrigin::list([]))
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::PUT,
                axum::http::Method::DELETE,
                axum::http::Method::OPTIONS,
            ])
            .allow_headers([
                axum::http::header::CONTENT_TYPE,
                axum::http::header::AUTHORIZATION,
                HeaderName::from_static("x-app-id"),
                HeaderName::from_static("x-app-key"),
            ])
    } else {
        let origins: Vec<axum::http::HeaderValue> = state
            .config
            .cors_origins
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::PUT,
                axum::http::Method::DELETE,
                axum::http::Method::OPTIONS,
            ])
            .allow_headers([
                axum::http::header::CONTENT_TYPE,
                axum::http::header::AUTHORIZATION,
                HeaderName::from_static("x-app-id"),
                HeaderName::from_static("x-app-key"),
            ])
    };

    // ── 公开路由（无需鉴权）─────────────────────────────────────────────────
    let public = Router::new()
        .route("/health", get(health))
        .route("/api/health", get(health))
        .route("/api/v1/collect", post(handlers::sdk::collect))
        .route_layer(DefaultBodyLimit::max(1024 * 1024)) // 1MB body limit for collect
        .route(
            "/api/v1/collect/health",
            get(|| async { Json(json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") })) }),
        )
        .route("/api/auth/register", post(handlers::auth::register))
        .route("/api/auth/login", post(handlers::auth::login))
        .route("/api/auth/refresh", post(handlers::auth::refresh))
        .route("/api/auth/sse-token", post(handlers::auth::sse_token))
        // SSE 端点（通过短生命周期 token 鉴权，不走 JWT middleware）
        .route("/api/dashboard/realtime", get(handlers::dashboard::realtime))
        .route("/api/tracking/live-events", get(handlers::track_analysis::live_events))
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit::rate_limit));

    // ── 受保护路由（需 JWT 鉴权）─────────────────────────────────────────────
    let protected = Router::new()
        // 用户 / 分组管理
        .route("/api/users", get(handlers::user::list))
        .route("/api/users", post(handlers::user::create))
        .route("/api/users/:id", get(handlers::user::detail))
        .route("/api/users/:id", put(handlers::user::update))
        .route("/api/users/:id", delete(handlers::user::remove))
        .route("/api/groups", get(handlers::group::list))
        .route("/api/groups", post(handlers::group::create))
        .route("/api/groups/:id", get(handlers::group::detail))
        .route("/api/groups/:id", put(handlers::group::update))
        .route("/api/groups/:id", delete(handlers::group::remove))
        // 项目管理
        .route("/api/projects", get(handlers::project::list))
        .route("/api/projects", post(handlers::project::create))
        .route("/api/projects/:id", get(handlers::project::detail))
        .route("/api/projects/:id", put(handlers::project::update))
        .route("/api/projects/:id", delete(handlers::project::remove))
        // 错误查询
        .route("/api/errors", get(handlers::error::list))
        .route("/api/errors/:id", get(handlers::error::detail))
        // 网络监控
        .route("/api/network", get(handlers::network::list))
        .route("/api/network/:id", get(handlers::network::detail))
        .route("/api/network/stats", get(handlers::network::stats))
        // 仪表盘
        .route("/api/dashboard/overview", get(handlers::dashboard::overview))
        // 埋点事件管理
        .route("/api/track/events", get(handlers::tracking::list_events))
        .route("/api/track/events/:event_name", get(handlers::tracking::event_detail))
        .route("/api/track/definitions", get(handlers::tracking::list_definitions))
        .route("/api/track/definitions", post(handlers::tracking::create_definition))
        .route("/api/track/definitions/:id", put(handlers::tracking::update_definition))
        .route("/api/track/definitions/:id", delete(handlers::tracking::delete_definition))
        .route("/api/track/properties", get(handlers::tracking::list_properties))
        .route("/api/track/analysis", get(handlers::track_analysis::event_analysis))
        // AI 分析
        .route("/api/ai/analyze/:error_id", post(handlers::ai_analysis::trigger))
        .route("/api/ai/analysis/:error_id", get(handlers::ai_analysis::get_result))
        .route("/api/ai/analyses", get(handlers::ai_analysis::list_analyses))
        .route("/api/ai/analyze-batch", post(handlers::ai_analysis::trigger_batch))
        // Source Map（multipart 上传，单独设置 50MB 限制）
        .route("/api/sourcemaps", post(handlers::sourcemap::upload))
        .route_layer(DefaultBodyLimit::max(50 * 1024 * 1024))
        .route("/api/sourcemaps", get(handlers::sourcemap::list))
        .route("/api/sourcemaps/:id", get(handlers::sourcemap::detail))
        .route("/api/sourcemaps/:id", delete(handlers::sourcemap::remove))
        // 告警
        .route("/api/alerts/rules", get(handlers::alert::list_rules))
        .route("/api/alerts/rules", post(handlers::alert::create_rule))
        .route("/api/alerts/rules/:id", put(handlers::alert::update_rule))
        .route("/api/alerts/rules/:id", delete(handlers::alert::delete_rule))
        .route("/api/alerts/logs", get(handlers::alert::list_logs))
        .route("/api/alerts/logs/:id", get(handlers::alert::log_detail))
        // 漏斗分析
        .route("/api/tracking/funnels", get(handlers::track_analysis::list_funnels))
        .route("/api/tracking/funnels", post(handlers::track_analysis::create_funnel))
        .route("/api/tracking/funnels/:id", get(handlers::track_analysis::get_funnel))
        .route("/api/tracking/funnels/:id", put(handlers::track_analysis::update_funnel))
        .route("/api/tracking/funnels/:id", delete(handlers::track_analysis::delete_funnel))
        .route("/api/tracking/funnels/:id/analyze", post(handlers::track_analysis::analyze_funnel))
        // 留存分析
        .route("/api/tracking/retentions", get(handlers::track_analysis::list_retentions))
        .route("/api/tracking/retentions", post(handlers::track_analysis::create_retention))
        .route(
            "/api/tracking/retentions/:id/analyze",
            post(handlers::track_analysis::analyze_retention),
        )
        // 用户画像
        .route("/api/tracking/users", get(handlers::track_users::list))
        .route("/api/tracking/users/:distinct_id", get(handlers::track_users::detail))
        .route("/api/tracking/users/:distinct_id/events", get(handlers::track_users::events))
        .layer(middleware::from_fn_with_state(state.clone(), auth::require_auth));

    public
        .merge(protected)
        .layer(middleware::from_fn(security_headers::security_headers))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024)) // 全局 2MB body limit
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "code": 0,
        "message": "ok",
        "data": {
            "status": "ok",
            "version": env!("CARGO_PKG_VERSION"),
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_clone() {
        let (tx, _) = broadcast::channel(1);
        let state = AppState {
            config: Config {
                database_url: "pg://localhost".into(),
                redis_url: "redis://localhost".into(),
                jwt_secret: "secret".into(),
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
        let cloned = state.clone();
        assert_eq!(cloned.config.server_port, 8080);
        assert!(cloned.db.is_none());
    }

    #[test]
    fn test_build_router_creates_routes() {
        let (tx, _) = broadcast::channel(1);
        let state = AppState {
            config: Config {
                database_url: String::new(),
                redis_url: String::new(),
                jwt_secret: "test".into(),
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
        let router = build_router(state);
        // Router should be buildable without panic
        let _ = router;
    }
}
