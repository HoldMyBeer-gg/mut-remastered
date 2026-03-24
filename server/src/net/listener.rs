use std::net::SocketAddr;

use sqlx::SqlitePool;
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn};

use crate::session::actor::ConnectionActor;

/// Shared application state cloned per connection.
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub session_ttl_secs: i64,
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
