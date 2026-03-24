mod auth;
mod config;
mod db;

use config::ServerConfig;
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

    // TCP listener will be added in Plan 02
    tracing::info!("server initialized (TCP listener not yet implemented)");

    // Keep pool alive for now; Plan 02 adds the accept loop
    drop(db_pool);
    Ok(())
}
