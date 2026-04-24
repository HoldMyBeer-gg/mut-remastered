//! WebSocket server endpoint for browser clients.
//!
//! Runs alongside the TCP listener on a separate port.
//! Browser clients connect via WebSocket and use the same binary protocol.
//! WebSocket messages are binary frames containing the same [ns][postcard] payload.

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use tower_http::cors::CorsLayer;
use tracing::{debug, info, warn};

use crate::net::listener::AppState;

/// Start the WebSocket server on the given address.
pub async fn run_ws_server(addr: &str, state: AppState) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(addr, "WebSocket server listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws_connection(socket, state))
}

/// Handle a WebSocket connection using an internal TCP loopback to reuse ConnectionActor.
///
/// Strategy: create a loopback TCP connection pair. One end feeds into ConnectionActor
/// (which expects TCP read/write halves). The other end bridges to/from the WebSocket.
/// WebSocket binary messages contain length-prefixed frames (same as raw TCP).
async fn handle_ws_connection(socket: WebSocket, state: AppState) {
    debug!("new WebSocket connection");

    // Split WebSocket into sender and receiver
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Create an in-process TCP loopback pair for the ConnectionActor
    let bridge_listener = match tokio::net::TcpListener::bind("127.0.0.1:0").await {
        Ok(l) => l,
        Err(e) => {
            warn!(error = %e, "failed to create WS bridge listener");
            return;
        }
    };
    let bridge_addr = bridge_listener.local_addr().unwrap();

    // Connect both sides of the bridge
    let (actor_stream_result, client_stream_result) = tokio::join!(
        tokio::net::TcpStream::connect(bridge_addr),
        bridge_listener.accept(),
    );

    let actor_stream = match actor_stream_result {
        Ok(s) => s,
        Err(e) => {
            warn!(error = %e, "WS bridge: actor connect failed");
            return;
        }
    };
    let (client_stream, _) = match client_stream_result {
        Ok(s) => s,
        Err(e) => {
            warn!(error = %e, "WS bridge: accept failed");
            return;
        }
    };

    let (actor_reader, actor_writer) = actor_stream.into_split();
    let (mut bridge_reader, mut bridge_writer) = client_stream.into_split();

    // Spawn the ConnectionActor on the actor side of the bridge
    let actor_state = state.clone();
    let actor_handle = tokio::spawn(async move {
        let mut actor =
            crate::session::actor::ConnectionActor::new(actor_reader, actor_writer, actor_state);
        if let Err(e) = actor.run().await {
            warn!(error = %e, "WS ConnectionActor error");
        }
        debug!("WS ConnectionActor finished");
    });

    // Task: WebSocket → Bridge (client sends binary frames)
    let ws_to_bridge = tokio::spawn(async move {
        use tokio::io::AsyncWriteExt;
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    // Binary data is the raw length-prefixed frame
                    if bridge_writer.write_all(&data).await.is_err() {
                        break;
                    }
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => continue,
            }
        }
        // Close the bridge write side to signal EOF to the actor
        drop(bridge_writer);
    });

    // Task: Bridge → WebSocket (actor sends binary frames, forward to WS)
    let bridge_to_ws = tokio::spawn(async move {
        use tokio::io::AsyncReadExt;
        let mut len_buf = [0u8; 4];
        loop {
            // Read length prefix
            match bridge_reader.read_exact(&mut len_buf).await {
                Ok(_) => {}
                Err(_) => break,
            }
            let payload_len = u32::from_le_bytes(len_buf) as usize;
            if payload_len > 64 * 1024 {
                break;
            }

            // Read payload
            let mut payload = vec![0u8; payload_len];
            if bridge_reader.read_exact(&mut payload).await.is_err() {
                break;
            }

            // Reconstruct full frame and send as WebSocket binary message
            let mut frame = Vec::with_capacity(4 + payload_len);
            frame.extend_from_slice(&len_buf);
            frame.extend_from_slice(&payload);

            if ws_sender.send(Message::Binary(frame.into())).await.is_err() {
                break;
            }
        }
    });

    // Wait for any task to finish (connection closed)
    tokio::select! {
        _ = actor_handle => {}
        _ = ws_to_bridge => {}
        _ = bridge_to_ws => {}
    }

    debug!("WebSocket connection closed");
}
