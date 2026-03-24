use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use tokio::sync::{RwLock, broadcast};

use server::combat::manager::CombatManager;
use server::combat::tick::{combat_tick_loop, load_monster_templates, spawn_initial_monsters};
use server::combat::types::SpawnEntry;
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

    // Collect spawn tables from zone data before moving world into Arc
    let mut spawn_tables: HashMap<RoomId, Vec<SpawnEntry>> = HashMap::new();
    for (room_id, room_def) in &world.rooms {
        if !room_def.spawns.is_empty() {
            spawn_tables.insert(room_id.clone(), room_def.spawns.clone());
        }
    }

    // Create per-room broadcast channels (capacity 32 per room)
    let mut room_channels_map: HashMap<RoomId, broadcast::Sender<WorldEvent>> = HashMap::new();
    for room_id in world.rooms.keys() {
        let (tx, _rx) = broadcast::channel::<WorldEvent>(32);
        room_channels_map.insert(room_id.clone(), tx);
    }

    // Load monster templates from data file
    let data_dir = Path::new(&config.worlds_dir).parent().unwrap_or(Path::new("..")).join("data");
    let monsters_path = data_dir.join("monsters.toml");
    let monster_templates = if monsters_path.exists() {
        let templates = load_monster_templates(&monsters_path)?;
        tracing::info!(templates = templates.len(), "monster templates loaded");
        templates
    } else {
        tracing::warn!("no monsters.toml found at {:?}", monsters_path);
        HashMap::new()
    };

    // Load item templates
    let items_path = data_dir.join("items.toml");
    let item_templates = if items_path.exists() {
        let templates = server::inventory::types::load_item_templates(&items_path)?;
        tracing::info!(templates = templates.len(), "item templates loaded");
        templates
    } else {
        tracing::warn!("no items.toml found at {:?}", items_path);
        HashMap::new()
    };

    // Spawn initial monsters
    let active_monsters = spawn_initial_monsters(&spawn_tables, &monster_templates);
    let total_monsters: usize = active_monsters.values().map(|v| v.len()).sum();
    tracing::info!(monsters = total_monsters, rooms = active_monsters.len(), "initial monsters spawned");

    let world = Arc::new(RwLock::new(world));
    let room_channels = Arc::new(RwLock::new(room_channels_map));
    let combat_manager = Arc::new(RwLock::new(CombatManager::new()));
    let monster_templates = Arc::new(monster_templates);
    let active_monsters = Arc::new(RwLock::new(active_monsters));
    let respawn_timers = Arc::new(RwLock::new(Vec::new()));
    let item_templates = Arc::new(item_templates);

    // Spawn combat tick loop (runs every 2 seconds in background)
    tokio::spawn(combat_tick_loop(
        combat_manager.clone(),
        active_monsters.clone(),
        world.clone(),
        room_channels.clone(),
        monster_templates.clone(),
        respawn_timers.clone(),
        db_pool.clone(),
    ));

    // Build shared application state
    let state = AppState {
        db: db_pool,
        session_ttl_secs: config.session_ttl_secs,
        world,
        room_channels,
        combat_manager,
        monster_templates,
        active_monsters,
        respawn_timers,
        item_templates,
    };

    // Start TCP accept loop — blocks until server shuts down
    run_listener(&config.bind_addr, state).await?;

    Ok(())
}
