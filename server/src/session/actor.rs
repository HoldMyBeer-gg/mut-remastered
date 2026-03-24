use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
    task,
};
use tracing::{debug, warn};

use protocol::auth::{ClientMsg, ErrorCode, ServerMsg};
use protocol::codec::{decode_message, encode_message};

use crate::auth::{
    hash::{hash_password, verify_password},
    session::{create_session, delete_session, lookup_account, register_account},
};
use crate::net::listener::AppState;

const MAX_FRAME_SIZE: usize = 64 * 1024; // 64 KiB

/// Per-connection actor that processes client messages and maintains session state.
///
/// The actor owns the TCP read/write halves and operates entirely within a single
/// Tokio task — no shared state besides the SqlitePool in AppState.
pub struct ConnectionActor {
    reader: OwnedReadHalf,
    writer: OwnedWriteHalf,
    state: AppState,
    /// None until the client successfully logs in.
    session_token: Option<String>,
    /// None until the client successfully logs in.
    account_id: Option<String>,
}

impl ConnectionActor {
    pub fn new(reader: OwnedReadHalf, writer: OwnedWriteHalf, state: AppState) -> Self {
        Self {
            reader,
            writer,
            state,
            session_token: None,
            account_id: None,
        }
    }

    /// Main loop: read length-prefixed frames, decode, dispatch, repeat.
    ///
    /// On clean EOF or any I/O error, breaks out and calls cleanup.
    pub async fn run(&mut self) -> anyhow::Result<()> {
        loop {
            match self.read_frame().await {
                Ok(Some(bytes)) => {
                    match decode_message::<ClientMsg>(&bytes) {
                        Ok(msg) => {
                            if let Err(e) = self.handle_message(msg).await {
                                warn!(error = %e, "error handling message");
                                // On internal errors keep going — send an Error response where possible
                            }
                        }
                        Err(e) => {
                            warn!(error = %e, "failed to decode client message");
                            let _ = self
                                .send(ServerMsg::Error {
                                    code: ErrorCode::InternalError,
                                    message: "invalid message encoding".to_string(),
                                })
                                .await;
                        }
                    }
                }
                Ok(None) => {
                    // Clean EOF
                    debug!("client disconnected (EOF)");
                    break;
                }
                Err(e) => {
                    warn!(error = %e, "read error");
                    break;
                }
            }
        }
        self.cleanup().await;
        Ok(())
    }

    /// Dispatch a decoded ClientMsg to the appropriate handler.
    async fn handle_message(&mut self, msg: ClientMsg) -> anyhow::Result<()> {
        match msg {
            ClientMsg::Register { username, password } => {
                // Hash password on a blocking thread — Argon2 is CPU-intensive
                let hash = task::spawn_blocking(move || hash_password(&password))
                    .await
                    .map_err(|e| anyhow::anyhow!("spawn_blocking join error: {e}"))??;

                match register_account(&self.state.db, &username, &hash).await {
                    Ok(account_id) => {
                        debug!(%username, %account_id, "account registered");
                        self.send(ServerMsg::RegisterOk { account_id }).await?;
                    }
                    Err(e) if e.to_string().contains("already taken") => {
                        self.send(ServerMsg::Error {
                            code: ErrorCode::UsernameTaken,
                            message: format!("username '{}' is already taken", username),
                        })
                        .await?;
                    }
                    Err(e) => {
                        warn!(error = %e, "register_account failed");
                        self.send(ServerMsg::Error {
                            code: ErrorCode::InternalError,
                            message: "registration failed".to_string(),
                        })
                        .await?;
                    }
                }
            }

            ClientMsg::Login { username, password } => {
                match lookup_account(&self.state.db, &username).await {
                    Ok(None) => {
                        self.send(ServerMsg::Error {
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
                            self.send(ServerMsg::Error {
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
                            self.account_id = Some(account_id);
                            self.send(ServerMsg::LoginOk {
                                session_token: token,
                            })
                            .await?;
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "lookup_account failed");
                        self.send(ServerMsg::Error {
                            code: ErrorCode::InternalError,
                            message: "login failed".to_string(),
                        })
                        .await?;
                    }
                }
            }

            ClientMsg::Logout => {
                if let Some(token) = self.session_token.take() {
                    if let Err(e) = delete_session(&self.state.db, &token).await {
                        warn!(error = %e, "delete_session failed on logout");
                    }
                    self.account_id = None;
                }
                self.send(ServerMsg::LogoutOk).await?;
            }

            ClientMsg::Ping => {
                self.send(ServerMsg::Pong).await?;
            }
        }
        Ok(())
    }

    /// Encode and write a ServerMsg to the TCP stream.
    async fn send(&mut self, msg: ServerMsg) -> anyhow::Result<()> {
        let bytes = encode_message(&msg)?;
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
    }

    /// Read a length-prefixed frame from the TCP stream.
    ///
    /// Reads 4 bytes as a LE u32 length, then reads that many payload bytes.
    /// Returns `None` on clean EOF (client disconnected), `Some(bytes)` on success.
    async fn read_frame(&mut self) -> anyhow::Result<Option<Vec<u8>>> {
        let mut len_buf = [0u8; 4];
        match self.reader.read_exact(&mut len_buf).await {
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
        self.reader.read_exact(&mut payload).await?;
        Ok(Some(payload))
    }
}
