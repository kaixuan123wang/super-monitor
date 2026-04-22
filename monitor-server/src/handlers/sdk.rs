//! SDK 数据上报接口 `/api/v1/collect`。
//!
//! Phase 1 提供占位实现：接收任意 JSON，直接响应 ok。
//! Phase 2 起按 `type` 字段（error / network / track / profile 等）分发到不同服务。

use axum::Json;
use serde_json::{json, Value};

pub async fn collect(Json(_payload): Json<Value>) -> Json<Value> {
    Json(json!({ "code": 0, "message": "ok", "data": null }))
}
