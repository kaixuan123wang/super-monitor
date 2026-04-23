use std::net::SocketAddr;

use anyhow::Context;
use migration::{Migrator, MigratorTrait};
use monitor_server::{
    config::Config,
    db, router,
    services::{alert_service, stats_service},
};
use tokio::sync::broadcast;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cfg = Config::from_env().context("failed to load config from env")?;
    tracing::info!(port = cfg.server_port, "starting monitor-server");

    // 数据库连接
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
            tracing::warn!(error = %e, "database connection failed, running without DB");
            None
        }
    };

    // 预聚合后台任务
    if let Some(ref conn) = db_conn {
        let conn_clone = conn.clone();
        tokio::spawn(async move {
            stats_service::start_aggregation_loop(conn_clone).await;
        });
        tracing::info!("stats aggregation background task started");
    }

    // 告警广播 channel（容量 128）
    let (alert_tx, _) = broadcast::channel::<alert_service::AlertEvent>(128);

    // 告警后台检查任务
    if let Some(ref conn) = db_conn {
        let conn_clone = conn.clone();
        let tx_clone = alert_tx.clone();
        tokio::spawn(async move {
            alert_service::start_alert_loop(conn_clone, tx_clone).await;
        });
        tracing::info!("alert background task started");
    }

    // sourcemap 目录
    if let Err(e) = tokio::fs::create_dir_all(&cfg.sourcemap_dir).await {
        tracing::warn!(error = %e, dir = %cfg.sourcemap_dir, "failed to create sourcemap dir");
    }

    let state = router::AppState {
        config: cfg.clone(),
        db: db_conn,
        alert_tx,
    };

    let app = router::build_router(state);

    let addr: SocketAddr = format!("0.0.0.0:{}", cfg.server_port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "monitor-server listening");
    axum::serve(listener, app).await?;
    Ok(())
}
