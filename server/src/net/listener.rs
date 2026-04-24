use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

use crate::combat::manager::CombatManager;
use crate::combat::types::{ActiveMonster, MonsterTemplate, RespawnTimer};
use crate::session::actor::ConnectionActor;
use crate::world::types::{RoomId, World, WorldEvent};

/// Shared application state cloned per connection.
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub session_ttl_secs: i64,
    /// In-memory world: rooms, room states, and player positions.
    /// Read-locked for queries; write-locked only on mutations (move, trigger).
    pub world: Arc<RwLock<World>>,
    /// Per-room broadcast senders. Players subscribe on login/move; events sent on triggers/moves.
    pub room_channels: Arc<RwLock<HashMap<RoomId, broadcast::Sender<WorldEvent>>>>,
    /// Combat manager: holds all active combat instances.
    pub combat_manager: Arc<RwLock<CombatManager>>,
    /// Monster templates loaded from TOML (immutable after startup).
    pub monster_templates: Arc<HashMap<String, MonsterTemplate>>,
    /// Active monster instances per room.
    pub active_monsters: Arc<RwLock<HashMap<RoomId, Vec<ActiveMonster>>>>,
    /// Respawn timers for dead monsters.
    pub respawn_timers: Arc<RwLock<Vec<RespawnTimer>>>,
    /// Item templates loaded from TOML (immutable after startup).
    pub item_templates: Arc<HashMap<String, crate::inventory::types::ItemTemplate>>,
    /// Global gossip broadcast channel (all online players subscribe).
    pub gossip_channel: broadcast::Sender<(String, String)>, // (sender_name, text)
}

/// Accept loop: binds TCP listener and spawns one independent task per connection.
///
/// Satisfies NETW-01: each connection runs in its own task; one player's actions
/// cannot block another.
pub async fn run_listener(addr: &str, state: AppState) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!(addr, "listening on {addr}");

    loop {
        match listener.accept().await {
            Ok((socket, peer_addr)) => {
                let state = state.clone();
                tokio::spawn(handle_connection(socket, peer_addr, state));
            }
            Err(e) => {
                warn!(error = %e, "accept error — continuing");
            }
        }
    }
}

/// Per-connection handler: splits TCP stream and runs the connection actor.
async fn handle_connection(
    socket: TcpStream,
    peer_addr: SocketAddr,
    state: AppState,
) -> anyhow::Result<()> {
    info!(%peer_addr, "new connection");
    let (reader, writer) = socket.into_split();
    let mut actor = ConnectionActor::new(reader, writer, state);
    let result = actor.run().await;
    info!(%peer_addr, "connection closed");
    result
}
