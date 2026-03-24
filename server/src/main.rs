mod auth;
mod config;
mod db;
mod net;
mod session;

use config::ServerConfig;
use net::listener::{AppState, run_listener};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("server=debug,tower_http=debug")),
        )
        .init();

    let config = ServerConfig::from_env();
    tracing::info!(bind_addr = %config.bind_addr, "starting MUT Remastered server");

    // Initialize database
    let db_pool = db::init_db(&config.database_url).await?;
    tracing::info!("database ready");

    // Build shared application state
    let state = AppState {
        db: db_pool,
        session_ttl_secs: config.session_ttl_secs,
    };

    // Start TCP accept loop — blocks until server shuts down
    run_listener(&config.bind_addr, state).await?;

    Ok(())
}
