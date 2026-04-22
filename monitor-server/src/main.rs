use std::net::SocketAddr;

use anyhow::Context;
use migration::{Migrator, MigratorTrait};
use monitor_server::{config::Config, db, router, services::stats_service};
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

    // 数据库连接（Phase 2 起：连接成功后自动运行迁移）
    let db_conn = match db::connect(&cfg.database_url).await {
        Ok(conn) => {
            tracing::info!("database connected");
            match Migrator::up(&conn, None).await {
                Ok(_) => tracing::info!("database migrations applied"),
                Err(e) => tracing::warn!(error = %e, "database migration failed"),
            }
            Some(conn)
        }
        Err(e) => {
            tracing::warn!(error = %e, "database connection failed, running without DB (collect endpoints will reject)");
            None
        }
    };

    // 启动预聚合后台任务（有 DB 连接时）
    if let Some(ref conn) = db_conn {
        let conn_clone = conn.clone();
        tokio::spawn(async move {
            stats_service::start_aggregation_loop(conn_clone).await;
        });
        tracing::info!("stats aggregation background task started");
    }

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
