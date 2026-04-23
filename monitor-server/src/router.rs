use axum::{
    middleware,
    routing::{delete, get, post, put},
    Json, Router,
};
use sea_orm::DatabaseConnection;
use serde_json::json;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::config::Config;
use crate::handlers;
use crate::middleware::{auth, rate_limit};
use crate::services::alert_service::AlertEvent;

/// 共享应用状态（通过 Axum Extension / State 传递）。
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: Option<DatabaseConnection>,
    pub alert_tx: broadcast::Sender<AlertEvent>,
}

/// 构建根 Router。
pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // ── 公开路由（无需鉴权）─────────────────────────────────────────────────
    let public = Router::new()
        .route("/health", get(health))
        .route("/api/health", get(health))
        .route("/api/v1/collect", post(handlers::sdk::collect))
        .route(
            "/api/v1/collect/health",
            get(|| async { Json(json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") })) }),
        )
        .route("/api/auth/register", post(handlers::auth::register))
        .route("/api/auth/login", post(handlers::auth::login))
        .route("/api/auth/refresh", post(handlers::auth::refresh))
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
        // 仪表盘 + SSE
        .route("/api/dashboard/overview", get(handlers::dashboard::overview))
        .route("/api/dashboard/realtime", get(handlers::dashboard::realtime))
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
        // Source Map
        .route("/api/sourcemaps", post(handlers::sourcemap::upload))
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
        .route("/api/tracking/retentions/:id/analyze", post(handlers::track_analysis::analyze_retention))
        // 实时事件流
        .route("/api/tracking/live-events", get(handlers::track_analysis::live_events))
        // 用户画像
        .route("/api/tracking/users", get(handlers::track_users::list))
        .route("/api/tracking/users/:distinct_id", get(handlers::track_users::detail))
        .route("/api/tracking/users/:distinct_id/events", get(handlers::track_users::events))
        .layer(middleware::from_fn_with_state(state.clone(), auth::require_auth));

    public
        .merge(protected)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
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
