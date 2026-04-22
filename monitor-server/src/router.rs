use axum::{routing::get, Json, Router};
use sea_orm::DatabaseConnection;
use serde_json::json;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::config::Config;

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
        // 健康检查 —— Phase 1 验收标准
        .route("/health", get(health))
        .route("/api/health", get(health))
        // SDK 数据上报（Phase 2 具体实现）
        .route("/api/v1/collect", axum::routing::post(crate::handlers::sdk::collect))
        .route("/api/v1/collect/health", get(|| async { Json(json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") })) }))
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
