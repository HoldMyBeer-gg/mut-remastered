use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use tokio::sync::{RwLock, broadcast};

use server::config::ServerConfig;
use server::net::listener::{AppState, run_listener};
use server::world::types::{RoomId, WorldEvent};
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
    let db_pool = server::db::init_db(&config.database_url).await?;
    tracing::info!("database ready");

    // Load world from zone TOML files and overlay persisted SQLite state
    let world = server::world::loader::load_world(Path::new(&config.worlds_dir), &db_pool).await?;
    tracing::info!(rooms = world.rooms.len(), "world loaded");

    // Create per-room broadcast channels (capacity 32 per room)
    let mut room_channels_map: HashMap<RoomId, broadcast::Sender<WorldEvent>> = HashMap::new();
    for room_id in world.rooms.keys() {
        let (tx, _rx) = broadcast::channel::<WorldEvent>(32);
        room_channels_map.insert(room_id.clone(), tx);
    }

    let world = Arc::new(RwLock::new(world));
    let room_channels = Arc::new(RwLock::new(room_channels_map));

    // Build shared application state
    let state = AppState {
        db: db_pool,
        session_ttl_secs: config.session_ttl_secs,
        world,
        room_channels,
    };

    // Start TCP accept loop — blocks until server shuts down
    run_listener(&config.bind_addr, state).await?;

    Ok(())
}
