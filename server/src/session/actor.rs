use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
    sync::broadcast,
    task,
};
use tracing::{debug, warn};

use protocol::auth::{ErrorCode, ServerMsg as AuthServerMsg};
use protocol::codec::{decode_message, encode_message, NS_AUTH, NS_WORLD};
use protocol::world::ServerMsg as WorldServerMsg;

use crate::auth::{
    hash::{hash_password, verify_password},
    session::{create_session, delete_session, lookup_account, register_account},
};
use crate::net::listener::AppState;
use crate::world::types::{RoomId, WorldEvent};

const MAX_FRAME_SIZE: usize = 64 * 1024; // 64 KiB

/// Default spawn room for players who have never logged in before.
const DEFAULT_SPAWN_ROOM: &str = "starting_village:market_square";

/// Per-connection actor that processes client messages and maintains session state.
///
/// The actor owns the TCP read/write halves and operates entirely within a single
/// Tokio task — no shared state besides the SqlitePool in AppState.
///
/// Extended in Phase 2 with:
/// - `room_receiver`: subscription to the current room's broadcast channel
/// - `tutorial_complete`: whether the player has finished the tutorial
/// - `tokio::select!` loop to handle both client frames and room broadcast events
pub struct ConnectionActor {
    reader: OwnedReadHalf,
    writer: OwnedWriteHalf,
    state: AppState,
    /// None until the client successfully logs in.
    session_token: Option<String>,
    /// None until the client successfully logs in.
    account_id: Option<String>,
    /// Subscription to the current room's broadcast channel.
    /// None when not logged in or room channel not found.
    room_receiver: Option<broadcast::Receiver<WorldEvent>>,
    /// Whether the player has completed the tutorial.
    /// Loaded from account_flags table on login.
    tutorial_complete: bool,
}

impl ConnectionActor {
    pub fn new(reader: OwnedReadHalf, writer: OwnedWriteHalf, state: AppState) -> Self {
        Self {
            reader,
            writer,
            state,
            session_token: None,
            account_id: None,
            room_receiver: None,
            tutorial_complete: false,
        }
    }

    /// Main loop: select on client frames and room broadcast events simultaneously.
    ///
    /// Uses tokio::select! so room events can be forwarded to the player while
    /// they are idle — satisfying WRLD-06 (players see events from other players).
    ///
    /// To satisfy the borrow checker, we split read_frame into a future over the
    /// reader only, and take a mutable reference to room_receiver separately.
    pub async fn run(&mut self) -> anyhow::Result<()> {
        loop {
            // Split borrows so tokio::select! can hold them concurrently.
            // The reader and room_receiver are independent fields.
            let frame_result;
            let event_result: Option<Result<WorldEvent, broadcast::error::RecvError>>;

            if let Some(rx) = &mut self.room_receiver {
                tokio::select! {
                    frame = read_frame_from(&mut self.reader) => {
                        frame_result = Some(frame);
                        event_result = None;
                    }
                    event = rx.recv() => {
                        frame_result = None;
                        event_result = Some(event);
                    }
                }
            } else {
                let frame = read_frame_from(&mut self.reader).await;
                frame_result = Some(frame);
                event_result = None;
            }

            if let Some(frame) = frame_result {
                match frame? {
                    Some(bytes) => {
                        if let Err(e) = self.dispatch_frame(&bytes).await {
                            warn!(error = %e, "error handling message");
                        }
                    }
                    None => {
                        debug!("client disconnected (EOF)");
                        break;
                    }
                }
            }

            if let Some(event) = event_result {
                match event {
                    Ok(world_event) => {
                        let msg = WorldServerMsg::WorldEvent {
                            message: world_event.message,
                        };
                        if let Err(e) = self.send_world(msg).await {
                            warn!(error = %e, "failed to send world event");
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!(missed = n, "lagged on room broadcast — player can re-look to resync");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        self.room_receiver = None;
                    }
                }
            }
        }
        self.cleanup().await;
        Ok(())
    }

    /// Dispatch a raw frame to auth or world message handlers.
    ///
    /// Tries auth::ClientMsg first; if that fails, tries world::ClientMsg.
    /// Both enum types use postcard — different variant indices make accidental
    /// cross-decoding extremely unlikely.
    async fn dispatch_frame(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        if let Ok(msg) = decode_message::<protocol::auth::ClientMsg>(NS_AUTH, bytes) {
            return self.handle_auth_message(msg).await;
        }
        if let Ok(msg) = decode_message::<protocol::world::ClientMsg>(NS_WORLD, bytes) {
            return self.handle_world_message(msg).await;
        }
        // Neither decoded successfully
        self.send_auth(AuthServerMsg::Error {
            code: ErrorCode::InternalError,
            message: "invalid message encoding".to_string(),
        })
        .await?;
        Ok(())
    }

    /// Dispatch a decoded auth ClientMsg to the appropriate handler.
    async fn handle_auth_message(
        &mut self,
        msg: protocol::auth::ClientMsg,
    ) -> anyhow::Result<()> {
        match msg {
            protocol::auth::ClientMsg::Register { username, password } => {
                // Hash password on a blocking thread — Argon2 is CPU-intensive
                let hash = task::spawn_blocking(move || hash_password(&password))
                    .await
                    .map_err(|e| anyhow::anyhow!("spawn_blocking join error: {e}"))??;

                match register_account(&self.state.db, &username, &hash).await {
                    Ok(account_id) => {
                        debug!(%username, %account_id, "account registered");
                        self.send_auth(AuthServerMsg::RegisterOk { account_id }).await?;
                    }
                    Err(e) if e.to_string().contains("already taken") => {
                        self.send_auth(AuthServerMsg::Error {
                            code: ErrorCode::UsernameTaken,
                            message: format!("username '{}' is already taken", username),
                        })
                        .await?;
                    }
                    Err(e) => {
                        warn!(error = %e, "register_account failed");
                        self.send_auth(AuthServerMsg::Error {
                            code: ErrorCode::InternalError,
                            message: "registration failed".to_string(),
                        })
                        .await?;
                    }
                }
            }

            protocol::auth::ClientMsg::Login { username, password } => {
                match lookup_account(&self.state.db, &username).await {
                    Ok(None) => {
                        self.send_auth(AuthServerMsg::Error {
                            code: ErrorCode::InvalidCredentials,
                            message: "invalid username or password".to_string(),
                        })
                        .await?;
                    }
                    Ok(Some((account_id, stored_hash))) => {
                        // Verify password on a blocking thread
                        let password_ok =
                            task::spawn_blocking(move || verify_password(&password, &stored_hash))
                                .await
                                .map_err(|e| anyhow::anyhow!("spawn_blocking join error: {e}"))??;

                        if !password_ok {
                            self.send_auth(AuthServerMsg::Error {
                                code: ErrorCode::InvalidCredentials,
                                message: "invalid username or password".to_string(),
                            })
                            .await?;
                        } else {
                            let token = create_session(
                                &self.state.db,
                                &account_id,
                                self.state.session_ttl_secs,
                            )
                            .await?;
                            debug!(%username, %account_id, "login successful");
                            self.session_token = Some(token.clone());
                            self.account_id = Some(account_id.clone());

                            // Place player in world if not already positioned
                            self.ensure_player_in_world(&account_id).await;

                            // Subscribe to room broadcast channel
                            self.subscribe_to_current_room(&account_id).await;

                            // Load tutorial completion flag
                            self.tutorial_complete =
                                self.load_tutorial_complete(&account_id).await;

                            self.send_auth(AuthServerMsg::LoginOk {
                                session_token: token,
                            })
                            .await?;
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "lookup_account failed");
                        self.send_auth(AuthServerMsg::Error {
                            code: ErrorCode::InternalError,
                            message: "login failed".to_string(),
                        })
                        .await?;
                    }
                }
            }

            protocol::auth::ClientMsg::Logout => {
                if let Some(token) = self.session_token.take() {
                    if let Err(e) = delete_session(&self.state.db, &token).await {
                        warn!(error = %e, "delete_session failed on logout");
                    }
                    self.account_id = None;
                    self.room_receiver = None;
                    self.tutorial_complete = false;
                }
                self.send_auth(AuthServerMsg::LogoutOk).await?;
            }

            protocol::auth::ClientMsg::Ping => {
                self.send_auth(AuthServerMsg::Pong).await?;
            }
        }
        Ok(())
    }

    /// Dispatch a decoded world ClientMsg to the appropriate command handler.
    async fn handle_world_message(
        &mut self,
        msg: protocol::world::ClientMsg,
    ) -> anyhow::Result<()> {
        // World commands require login
        let account_id = match &self.account_id {
            Some(id) => id.clone(),
            None => {
                self.send_auth(AuthServerMsg::Error {
                    code: ErrorCode::SessionExpired,
                    message: "you must log in first".to_string(),
                })
                .await?;
                return Ok(());
            }
        };

        match msg {
            protocol::world::ClientMsg::Look => {
                let resp = crate::world::commands::handle_look(
                    &self.state.world,
                    &account_id,
                    self.tutorial_complete,
                )
                .await;
                self.send_world(resp).await?;
            }

            protocol::world::ClientMsg::Move { direction } => {
                let (resp, auto_look, new_room_id) = crate::world::commands::handle_move(
                    &self.state.world,
                    &self.state.room_channels,
                    &self.state.db,
                    &account_id,
                    &direction,
                    self.tutorial_complete,
                )
                .await;
                self.send_world(resp).await?;
                if let Some(room_desc) = auto_look {
                    self.send_world(room_desc).await?;
                }
                // Re-subscribe to new room's broadcast channel on successful move
                if let Some(new_room) = new_room_id {
                    let channels = self.state.room_channels.read().await;
                    if let Some(sender) = channels.get(&new_room) {
                        self.room_receiver = Some(sender.subscribe());
                    }
                }
            }

            protocol::world::ClientMsg::Examine { target } => {
                let resp = crate::world::commands::handle_examine(
                    &self.state.world,
                    &account_id,
                    &target,
                )
                .await;
                self.send_world(resp).await?;
            }

            protocol::world::ClientMsg::Interact { command } => {
                let (responses, tutorial_now_complete) =
                    crate::world::commands::handle_interact(
                        &self.state.world,
                        &self.state.room_channels,
                        &self.state.db,
                        &account_id,
                        &command,
                    )
                    .await;
                for resp in responses {
                    self.send_world(resp).await?;
                }
                if tutorial_now_complete {
                    self.tutorial_complete = true;
                }
            }
        }
        Ok(())
    }

    /// Encode and write an auth ServerMsg to the TCP stream.
    async fn send_auth(&mut self, msg: AuthServerMsg) -> anyhow::Result<()> {
        let bytes = encode_message(NS_AUTH, &msg)?;
        self.writer.write_all(&bytes).await?;
        Ok(())
    }

    /// Encode and write a world ServerMsg to the TCP stream.
    async fn send_world(&mut self, msg: WorldServerMsg) -> anyhow::Result<()> {
        let bytes = encode_message(NS_WORLD, &msg)?;
        self.writer.write_all(&bytes).await?;
        Ok(())
    }

    /// Graceful cleanup on connection drop: delete the active session if any.
    ///
    /// Satisfies AUTH-08: disconnecting without explicit logout still invalidates the session.
    async fn cleanup(&mut self) {
        if let Some(token) = self.session_token.take() {
            if let Err(e) = delete_session(&self.state.db, &token).await {
                warn!(error = %e, "delete_session failed during cleanup");
            }
            self.account_id = None;
        }
        self.room_receiver = None;
    }

    /// Ensure the player has a position in the world. If not, place them at the default spawn.
    ///
    /// The spawn room is determined by (in priority order):
    /// 1. `world.default_spawn` — set at runtime by test helpers for custom spawn locations
    /// 2. `DEFAULT_SPAWN_ROOM` constant — the production default
    async fn ensure_player_in_world(&self, account_id: &str) {
        let (needs_placement, spawn_override) = {
            let w = self.state.world.read().await;
            (!w.player_positions.contains_key(account_id), w.default_spawn.clone())
        };

        if needs_placement {
            let spawn = spawn_override
                .unwrap_or_else(|| RoomId(DEFAULT_SPAWN_ROOM.to_string()));

            // Insert into in-memory world
            {
                let mut w = self.state.world.write().await;
                w.player_positions
                    .insert(account_id.to_string(), spawn.clone());
            }

            // Persist to SQLite
            if let Err(e) = sqlx::query(
                "INSERT OR IGNORE INTO player_positions (account_id, room_id, updated_at) VALUES (?, ?, unixepoch())"
            )
            .bind(account_id)
            .bind(&spawn.0)
            .execute(&self.state.db)
            .await
            {
                warn!(error = %e, "failed to persist initial player position");
            }
        }
    }

    /// Subscribe the actor's room_receiver to the player's current room channel.
    async fn subscribe_to_current_room(&mut self, account_id: &str) {
        let room_id = {
            let w = self.state.world.read().await;
            w.player_positions.get(account_id).cloned()
        };

        if let Some(room_id) = room_id {
            let channels = self.state.room_channels.read().await;
            if let Some(sender) = channels.get(&room_id) {
                self.room_receiver = Some(sender.subscribe());
            }
        }
    }

    /// Query the account_flags table to determine if the player has completed the tutorial.
    async fn load_tutorial_complete(&self, account_id: &str) -> bool {
        let result: Result<Option<(i64,)>, _> = sqlx::query_as(
            "SELECT 1 FROM account_flags WHERE account_id = ? AND flag = 'tutorial_complete'"
        )
        .bind(account_id)
        .fetch_optional(&self.state.db)
        .await;

        matches!(result, Ok(Some(_)))
    }

}

/// Read a length-prefixed frame from a TCP read half.
///
/// Free function (not a method) so it can be used inside tokio::select! alongside
/// &mut room_receiver without triggering E0500 double-borrow on the actor struct.
///
/// Reads 4 bytes as a LE u32 length, then reads that many payload bytes.
/// Returns `None` on clean EOF (client disconnected), `Some(bytes)` on success.
async fn read_frame_from(
    reader: &mut OwnedReadHalf,
) -> anyhow::Result<Option<Vec<u8>>> {
    let mut len_buf = [0u8; 4];
    match reader.read_exact(&mut len_buf).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            return Ok(None); // clean disconnect
        }
        Err(e) => return Err(e.into()),
    }

    let payload_len = u32::from_le_bytes(len_buf) as usize;
    if payload_len > MAX_FRAME_SIZE {
        return Err(anyhow::anyhow!(
            "frame too large: {} bytes (max {})",
            payload_len,
            MAX_FRAME_SIZE
        ));
    }

    let mut payload = vec![0u8; payload_len];
    reader.read_exact(&mut payload).await?;
    Ok(Some(payload))
}
