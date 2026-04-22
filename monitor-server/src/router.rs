use axum::{
    routing::{delete, get, post, put},
    Json, Router,
};
use sea_orm::DatabaseConnection;
use serde_json::json;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::config::Config;
use crate::handlers;

/// 共享应用状态（通过 Axum Extension / State 传递）。
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: Option<DatabaseConnection>,
}

/// 构建根 Router。
pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // 健康检查
        .route("/health", get(health))
        .route("/api/health", get(health))
        // SDK 数据上报
        .route("/api/v1/collect", post(handlers::sdk::collect))
        .route(
            "/api/v1/collect/health",
            get(|| async { Json(json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") })) }),
        )
        // 项目管理
        .route("/api/projects", get(handlers::project::list))
        .route("/api/projects", post(handlers::project::create))
        .route("/api/projects/:id", get(handlers::project::detail))
        .route("/api/projects/:id", put(handlers::project::update))
        .route("/api/projects/:id", delete(handlers::project::remove))
        // 错误查询
        .route("/api/errors", get(handlers::error::list))
        .route("/api/errors/:id", get(handlers::error::detail))
        // Phase 3 ── 仪表盘 + SSE
        .route("/api/dashboard/overview", get(handlers::dashboard::overview))
        .route("/api/dashboard/realtime", get(handlers::dashboard::realtime))
        // Phase 3 ── 已采集事件
        .route("/api/track/events", get(handlers::tracking::list_events))
        .route("/api/track/events/:event_name", get(handlers::tracking::event_detail))
        // Phase 3 ── 自定义事件定义 CRUD
        .route("/api/track/definitions", get(handlers::tracking::list_definitions))
        .route("/api/track/definitions", post(handlers::tracking::create_definition))
        .route("/api/track/definitions/:id", put(handlers::tracking::update_definition))
        .route("/api/track/definitions/:id", delete(handlers::tracking::delete_definition))
        // Phase 3 ── 属性汇总
        .route("/api/track/properties", get(handlers::tracking::list_properties))
        // Phase 3 ── 埋点事件分析
        .route("/api/track/analysis", get(handlers::track_analysis::event_analysis))
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
