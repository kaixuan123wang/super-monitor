//! HTTP 处理器（每个文件对应一个资源域）。Phase 1 声明模块，具体实现在 Phase 2+。

pub mod ai_analysis;
pub mod alert;
pub mod auth;
pub mod dashboard;
pub mod error;
pub mod group;
pub mod network;
pub mod project;
pub mod sdk;
pub mod sourcemap;
pub mod track_analysis;
pub mod track_users;
pub mod tracking;
pub mod user;
