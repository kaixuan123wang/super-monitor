//! monitor-server library root.
//!
//! Phase 1 目标：搭建基础骨架（配置、DB 连接、路由、健康检查、日志 / CORS）。
//! 后续阶段会在 `handlers` / `services` / `models` 中补齐业务实现。

pub mod config;
pub mod db;
pub mod router;
pub mod middleware;
pub mod handlers;
pub mod services;
pub mod models;
pub mod error;
