use std::net::SocketAddr;

use anyhow::Context;
use monitor_server::{config::Config, db, router};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // .env 优先加载（开发环境）
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cfg = Config::from_env().context("failed to load config from env")?;
    tracing::info!(port = cfg.server_port, "starting monitor-server");

    // 数据库连接（Phase 1 允许失败后打印警告，不阻塞 /health）
    let db_conn = match db::connect(&cfg.database_url).await {
        Ok(conn) => {
            tracing::info!("database connected");
            Some(conn)
        }
        Err(e) => {
            tracing::warn!(error = %e, "database connection failed, running without DB (Phase 1 only)");
            None
        }
    };

    let state = router::AppState {
        config: cfg.clone(),
        db: db_conn,
    };

    let app = router::build_router(state);

    let addr: SocketAddr = format!("0.0.0.0:{}", cfg.server_port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "monitor-server listening");
    axum::serve(listener, app).await?;
    Ok(())
}
