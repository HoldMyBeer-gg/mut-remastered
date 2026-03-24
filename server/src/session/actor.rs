use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
    sync::broadcast,
    task,
};
use tracing::{debug, warn};

use protocol::auth::{ErrorCode, ServerMsg as AuthServerMsg};
use protocol::codec::{decode_message, encode_message, NS_AUTH, NS_CHAR, NS_COMBAT, NS_WORLD};
use protocol::combat::ServerMsg as CombatServerMsg;
use protocol::world::ServerMsg as WorldServerMsg;

use crate::auth::{
    hash::{hash_password, verify_password},
    session::{create_session, delete_session, lookup_account, register_account},
};
use crate::character::creation::{calculate_initial_stats, validate_name, validate_point_buy};
use crate::character::types::{Class, Gender, Race};
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
    /// Active character ID (set after CharacterSelect).
    active_character_id: Option<String>,
    /// Active character name (for display in room/broadcasts).
    active_character_name: Option<String>,
    /// Subscription to the current room's broadcast channel.
    /// None when not logged in or room channel not found.
    room_receiver: Option<broadcast::Receiver<WorldEvent>>,
    /// Whether the player has completed the tutorial.
    /// Loaded from account_flags table on login.
    tutorial_complete: bool,
    /// Subscription to the global gossip broadcast channel.
    gossip_receiver: Option<broadcast::Receiver<(String, String)>>,
}

impl ConnectionActor {
    pub fn new(reader: OwnedReadHalf, writer: OwnedWriteHalf, state: AppState) -> Self {
        Self {
            reader,
            writer,
            state,
            session_token: None,
            account_id: None,
            active_character_id: None,
            active_character_name: None,
            room_receiver: None,
            tutorial_complete: false,
            gossip_receiver: None,
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

            // Drain gossip messages (non-blocking)
            let mut gossip_msgs = Vec::new();
            if let Some(ref mut gossip_rx) = self.gossip_receiver {
                while let Ok((sender, text)) = gossip_rx.try_recv() {
                    gossip_msgs.push((sender, text));
                }
            }
            for (sender, text) in gossip_msgs {
                let msg = WorldServerMsg::ChatMessage {
                    channel: "gossip".to_string(),
                    sender,
                    text,
                };
                if let Err(e) = self.send_world(msg).await {
                    warn!(error = %e, "failed to send gossip message");
                    break;
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
        if let Ok(msg) = decode_message::<protocol::character::ClientMsg>(NS_CHAR, bytes) {
            return self.handle_character_message(msg).await;
        }
        if let Ok(msg) = decode_message::<protocol::combat::ClientMsg>(NS_COMBAT, bytes) {
            return self.handle_combat_message(msg).await;
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

                            // Load tutorial completion flag
                            self.tutorial_complete =
                                self.load_tutorial_complete(&account_id).await;

                            self.send_auth(AuthServerMsg::LoginOk {
                                session_token: token,
                            })
                            .await?;

                            // NOTE: Player is NOT placed in world until CharacterSelect.
                            // The client should send CharacterList then CharacterSelect.
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
                // Remove character from world
                if let Some(char_id) = self.active_character_id.take() {
                    let mut w = self.state.world.write().await;
                    w.player_positions.remove(&char_id);
                    w.player_names.remove(&char_id);
                }
                self.active_character_name = None;

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
        // World commands require login AND character selection
        let character_id = match &self.active_character_id {
            Some(id) => id.clone(),
            None => {
                self.send_auth(AuthServerMsg::Error {
                    code: ErrorCode::SessionExpired,
                    message: "you must select a character first".to_string(),
                })
                .await?;
                return Ok(());
            }
        };

        match msg {
            protocol::world::ClientMsg::Look => {
                let resp = crate::world::commands::handle_look(
                    &self.state.world,
                    &self.state.active_monsters,
                    &character_id,
                    self.tutorial_complete,
                )
                .await;
                self.send_world(resp).await?;
            }

            protocol::world::ClientMsg::Move { direction } => {
                let (resp, auto_look, new_room_id) = crate::world::commands::handle_move(
                    &self.state.world,
                    &self.state.active_monsters,
                    &self.state.room_channels,
                    &self.state.db,
                    &character_id,
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
                    {
                        let channels = self.state.room_channels.read().await;
                        if let Some(sender) = channels.get(&new_room) {
                            self.room_receiver = Some(sender.subscribe());
                        }
                    } // channels guard dropped here
                    // Check for aggressive monsters in the new room
                    self.check_aggro(&new_room).await;
                }
            }

            protocol::world::ClientMsg::Examine { target } => {
                let resp = crate::world::commands::handle_examine(
                    &self.state.world,
                    &character_id,
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
                        &self.state.active_monsters,
                        &self.state.monster_templates,
                        &self.state.db,
                        &character_id,
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

            protocol::world::ClientMsg::Inventory => {
                let resp = crate::inventory::commands::handle_inventory(
                    &self.state.db,
                    &character_id,
                    &self.state.item_templates,
                )
                .await;
                self.send_world(resp).await?;
            }

            protocol::world::ClientMsg::GetItem { target } => {
                let room_id_str = {
                    let w = self.state.world.read().await;
                    w.player_positions.get(&character_id).map(|r| r.0.clone())
                };
                if let Some(room_id) = room_id_str {
                    let resp = crate::inventory::commands::handle_get_item(
                        &self.state.db,
                        &character_id,
                        &room_id,
                        &target,
                        &self.state.item_templates,
                    )
                    .await;
                    self.send_world(resp).await?;
                }
            }

            protocol::world::ClientMsg::DropItem { target } => {
                let room_id_str = {
                    let w = self.state.world.read().await;
                    w.player_positions.get(&character_id).map(|r| r.0.clone())
                };
                if let Some(room_id) = room_id_str {
                    let resp = crate::inventory::commands::handle_drop_item(
                        &self.state.db,
                        &character_id,
                        &room_id,
                        &target,
                        &self.state.item_templates,
                    )
                    .await;
                    self.send_world(resp).await?;
                }
            }

            protocol::world::ClientMsg::Equip { item_name } => {
                let resp = crate::inventory::commands::handle_equip(
                    &self.state.db,
                    &character_id,
                    &item_name,
                    &self.state.item_templates,
                )
                .await;
                self.send_world(resp).await?;
            }

            protocol::world::ClientMsg::Unequip { slot } => {
                let resp = crate::inventory::commands::handle_unequip(
                    &self.state.db,
                    &character_id,
                    &slot,
                    &self.state.item_templates,
                )
                .await;
                self.send_world(resp).await?;
            }

            protocol::world::ClientMsg::Stats => {
                let resp = crate::inventory::commands::handle_stats(
                    &self.state.db,
                    &character_id,
                    &self.state.item_templates,
                )
                .await;
                self.send_world(resp).await?;
            }

            protocol::world::ClientMsg::Bio { text } => {
                let resp = crate::inventory::commands::handle_bio(
                    &self.state.db,
                    &character_id,
                    &text,
                )
                .await;
                self.send_world(resp).await?;
            }

            protocol::world::ClientMsg::Say { text } => {
                let name = self.active_character_name.clone().unwrap_or_default();
                // Broadcast to room via room channel
                let room_id = {
                    let w = self.state.world.read().await;
                    w.player_positions.get(&character_id).cloned()
                };
                if let Some(room_id) = room_id {
                    let channels = self.state.room_channels.read().await;
                    if let Some(sender) = channels.get(&room_id) {
                        let _ = sender.send(crate::world::types::WorldEvent {
                            message: format!("[IC] {} says: {}", name, text),
                        });
                    }
                }
            }

            protocol::world::ClientMsg::Emote { text } => {
                let name = self.active_character_name.clone().unwrap_or_default();
                let room_id = {
                    let w = self.state.world.read().await;
                    w.player_positions.get(&character_id).cloned()
                };
                if let Some(room_id) = room_id {
                    let channels = self.state.room_channels.read().await;
                    if let Some(sender) = channels.get(&room_id) {
                        let _ = sender.send(crate::world::types::WorldEvent {
                            message: format!("[IC] {} {}", name, text),
                        });
                    }
                }
            }

            protocol::world::ClientMsg::Whisper { target, text } => {
                let name = self.active_character_name.clone().unwrap_or_default();
                // Find target in same room — for now send as room event with target prefix
                // A proper whisper would need direct-to-actor messaging; use room broadcast with filtering
                let room_id = {
                    let w = self.state.world.read().await;
                    w.player_positions.get(&character_id).cloned()
                };
                if let Some(room_id) = room_id {
                    let channels = self.state.room_channels.read().await;
                    if let Some(sender) = channels.get(&room_id) {
                        let _ = sender.send(crate::world::types::WorldEvent {
                            message: format!("[WHISPER] {} whispers to {}: {}", name, target, text),
                        });
                    }
                }
                self.send_world(WorldServerMsg::WhisperSent {
                    to: target,
                    text,
                })
                .await?;
            }

            protocol::world::ClientMsg::Gossip { text } => {
                let name = self.active_character_name.clone().unwrap_or_default();
                let _ = self.state.gossip_channel.send((name, text));
            }

            protocol::world::ClientMsg::ToggleChannel { channel } => {
                let resp = crate::social::commands::handle_toggle_channel(
                    &self.state.db,
                    &character_id,
                    &channel,
                )
                .await;
                self.send_world(resp).await?;
            }

            protocol::world::ClientMsg::LookAt { target } => {
                let resp = crate::social::commands::handle_look_at(
                    &self.state.db,
                    &self.state.world,
                    &character_id,
                    &target,
                    &self.state.item_templates,
                )
                .await;
                self.send_world(resp).await?;
            }

            protocol::world::ClientMsg::SetDescription { text } => {
                let resp = crate::social::commands::handle_set_description(
                    &self.state.db,
                    &character_id,
                    &text,
                )
                .await;
                self.send_world(resp).await?;
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

    /// Encode and write a character ServerMsg to the TCP stream.
    async fn send_char(
        &mut self,
        msg: protocol::character::ServerMsg,
    ) -> anyhow::Result<()> {
        let bytes = encode_message(NS_CHAR, &msg)?;
        self.writer.write_all(&bytes).await?;
        Ok(())
    }

    /// Handle character management messages (list, create, select).
    async fn handle_character_message(
        &mut self,
        msg: protocol::character::ClientMsg,
    ) -> anyhow::Result<()> {
        // Character commands require login
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
            protocol::character::ClientMsg::CharacterList => {
                let rows: Vec<(String, String, String, String, i64)> = sqlx::query_as(
                    "SELECT id, name, race, class, level FROM characters WHERE account_id = ? ORDER BY created_at"
                )
                .bind(&account_id)
                .fetch_all(&self.state.db)
                .await?;

                let characters = rows
                    .into_iter()
                    .map(|(id, name, race, class, level)| {
                        protocol::character::CharacterSummary {
                            id,
                            name,
                            race,
                            class,
                            level: level as u32,
                        }
                    })
                    .collect();

                self.send_char(protocol::character::ServerMsg::CharacterListResult {
                    characters,
                })
                .await?;
            }

            protocol::character::ClientMsg::CharacterCreate {
                name,
                race,
                class,
                gender,
                ability_scores,
                racial_bonus_choices,
            } => {
                // Validate name
                if let Err(reason) = validate_name(&name) {
                    self.send_char(protocol::character::ServerMsg::CharacterCreateFail {
                        reason,
                    })
                    .await?;
                    return Ok(());
                }

                // Parse and validate race
                let race_enum = match Race::from_str(&race) {
                    Some(r) => r,
                    None => {
                        self.send_char(protocol::character::ServerMsg::CharacterCreateFail {
                            reason: format!("Unknown race: '{}'. Valid races: human, elf, dwarf, halfling, orc, gnome, half_elf, tiefling", race),
                        })
                        .await?;
                        return Ok(());
                    }
                };

                // Validate racial bonus choices
                if let Err(reason) = race_enum.validate_choices(&racial_bonus_choices) {
                    self.send_char(protocol::character::ServerMsg::CharacterCreateFail {
                        reason,
                    })
                    .await?;
                    return Ok(());
                }

                // Parse and validate class
                let class_enum = match Class::from_str(&class) {
                    Some(c) => c,
                    None => {
                        self.send_char(protocol::character::ServerMsg::CharacterCreateFail {
                            reason: format!("Unknown class: '{}'. Valid classes: warrior, ranger, cleric, mage, rogue", class),
                        })
                        .await?;
                        return Ok(());
                    }
                };

                // Parse and validate gender
                let gender_enum = match Gender::from_str(&gender) {
                    Some(g) => g,
                    None => {
                        self.send_char(protocol::character::ServerMsg::CharacterCreateFail {
                            reason: format!("Unknown gender: '{}'. Valid options: male, female, non_binary", gender),
                        })
                        .await?;
                        return Ok(());
                    }
                };

                // Validate point buy
                if let Err(reason) = validate_point_buy(&ability_scores) {
                    self.send_char(protocol::character::ServerMsg::CharacterCreateFail {
                        reason,
                    })
                    .await?;
                    return Ok(());
                }

                // Calculate initial stats
                let stats = calculate_initial_stats(
                    &race_enum,
                    &class_enum,
                    &ability_scores,
                    &racial_bonus_choices,
                );

                let character_id = uuid::Uuid::new_v4().to_string();

                // Insert into DB
                let result = sqlx::query(
                    "INSERT INTO characters (id, account_id, name, race, class, gender, hp, max_hp, mana, max_mana, stamina, max_stamina, str_score, dex_score, con_score, int_score, wis_score, cha_score) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
                )
                .bind(&character_id)
                .bind(&account_id)
                .bind(&name)
                .bind(race_enum.as_str())
                .bind(class_enum.as_str())
                .bind(gender_enum.as_str())
                .bind(stats.hp)
                .bind(stats.max_hp)
                .bind(stats.mana)
                .bind(stats.max_mana)
                .bind(stats.stamina)
                .bind(stats.max_stamina)
                .bind(stats.final_scores[0] as i32)
                .bind(stats.final_scores[1] as i32)
                .bind(stats.final_scores[2] as i32)
                .bind(stats.final_scores[3] as i32)
                .bind(stats.final_scores[4] as i32)
                .bind(stats.final_scores[5] as i32)
                .execute(&self.state.db)
                .await;

                match result {
                    Ok(_) => {
                        debug!(%character_id, %name, "character created");
                        self.send_char(protocol::character::ServerMsg::CharacterCreateOk {
                            character_id,
                            name,
                        })
                        .await?;
                    }
                    Err(e) if e.to_string().contains("UNIQUE") => {
                        self.send_char(protocol::character::ServerMsg::CharacterCreateFail {
                            reason: format!("Character name '{}' is already taken", name),
                        })
                        .await?;
                    }
                    Err(e) => {
                        warn!(error = %e, "character creation failed");
                        self.send_char(protocol::character::ServerMsg::CharacterCreateFail {
                            reason: "Character creation failed".to_string(),
                        })
                        .await?;
                    }
                }
            }

            protocol::character::ClientMsg::CharacterSelect { character_id } => {
                // Verify character belongs to this account
                let row: Option<(String, String, String, i64, i64, i64, i64, i64, i64, i64)> = sqlx::query_as(
                    "SELECT name, race, class, hp, max_hp, mana, max_mana, stamina, max_stamina, level FROM characters WHERE id = ? AND account_id = ?"
                )
                .bind(&character_id)
                .bind(&account_id)
                .fetch_optional(&self.state.db)
                .await?;

                let (name, _race, _class, _hp, _max_hp, _mana, _max_mana, _stamina, _max_stamina, _level) = match row {
                    Some(r) => r,
                    None => {
                        self.send_char(protocol::character::ServerMsg::CharacterSelectFail {
                            reason: "Character not found or does not belong to your account".to_string(),
                        })
                        .await?;
                        return Ok(());
                    }
                };

                // Set active character
                self.active_character_id = Some(character_id.clone());
                self.active_character_name = Some(name.clone());

                // Place character in world
                self.ensure_character_in_world(&character_id).await;

                // Register display name
                {
                    let mut w = self.state.world.write().await;
                    w.player_names.insert(character_id.clone(), name.clone());
                }

                // Subscribe to room broadcast channel
                self.subscribe_to_current_room(&character_id).await;

                debug!(%character_id, %name, "character selected");

                self.send_char(protocol::character::ServerMsg::CharacterSelected {
                    character_id: character_id.clone(),
                    name: name.clone(),
                })
                .await?;

                // Send initial room description
                let room_desc = crate::world::commands::handle_look(
                    &self.state.world,
                    &self.state.active_monsters,
                    &character_id,
                    self.tutorial_complete,
                )
                .await;
                self.send_world(room_desc).await?;

                // Send initial vitals (CHAR-01)
                self.send_vitals(&character_id).await?;

                // Subscribe to global gossip channel
                self.gossip_receiver = Some(self.state.gossip_channel.subscribe());
            }
        }
        Ok(())
    }

    /// Encode and write a combat ServerMsg to the TCP stream.
    async fn send_combat(&mut self, msg: CombatServerMsg) -> anyhow::Result<()> {
        let bytes = encode_message(NS_COMBAT, &msg)?;
        self.writer.write_all(&bytes).await?;
        Ok(())
    }

    /// Handle combat messages (attack, flee, use ability).
    async fn handle_combat_message(
        &mut self,
        msg: protocol::combat::ClientMsg,
    ) -> anyhow::Result<()> {
        use crate::combat::engine::roll_initiative;
        use crate::combat::types::*;

        debug!(?msg, "received combat message");

        let character_id = match &self.active_character_id {
            Some(id) => id.clone(),
            None => {
                self.send_combat(CombatServerMsg::ActionFail {
                    reason: "you must select a character first".to_string(),
                })
                .await?;
                return Ok(());
            }
        };
        let character_name = self
            .active_character_name
            .clone()
            .unwrap_or_else(|| character_id.clone());

        // Get current room
        let room_id = {
            let w = self.state.world.read().await;
            w.player_positions.get(&character_id).cloned()
        };
        let room_id = match room_id {
            Some(r) => r,
            None => {
                self.send_combat(CombatServerMsg::ActionFail {
                    reason: "you are not in the world".to_string(),
                })
                .await?;
                return Ok(());
            }
        };

        match msg {
            protocol::combat::ClientMsg::Attack { target } => {
                // Find the target monster in this room
                let monster_info = {
                    let monsters = self.state.active_monsters.read().await;
                    if let Some(room_monsters) = monsters.get(&room_id) {
                        room_monsters
                            .iter()
                            .find(|m| {
                                m.is_alive()
                                    && m.name.to_lowercase().contains(&target.to_lowercase())
                            })
                            .map(|m| (m.id.clone(), m.name.clone()))
                    } else {
                        None
                    }
                };

                let (monster_id, monster_name) = match monster_info {
                    Some(info) => info,
                    None => {
                        self.send_combat(CombatServerMsg::ActionFail {
                            reason: format!("No target '{}' found here.", target),
                        })
                        .await?;
                        return Ok(());
                    }
                };

                // Check if already in combat in this room
                let already_in_combat = {
                    let mgr = self.state.combat_manager.read().await;
                    mgr.has_combat(&room_id)
                };

                if already_in_combat {
                    // Queue attack action in existing combat
                    let mut mgr = self.state.combat_manager.write().await;
                    mgr.queue_action(
                        &room_id,
                        CombatantId::Player(character_id.clone()),
                        CombatAction::Attack {
                            target: CombatantId::Monster(monster_id),
                        },
                    );
                } else {
                    // Start new combat
                    let dex_score = self.load_character_dex(&character_id).await;
                    let dex_mod = crate::character::types::ability_modifier(dex_score);

                    let combatants = vec![
                        CombatantInfo {
                            id: CombatantId::Player(character_id.clone()),
                            name: character_name.clone(),
                            initiative: roll_initiative(dex_mod),
                        },
                        CombatantInfo {
                            id: CombatantId::Monster(monster_id.clone()),
                            name: monster_name.clone(),
                            initiative: roll_initiative(0), // Monsters use 0 DEX mod for simplicity
                        },
                    ];

                    let names = {
                        let mut mgr = self.state.combat_manager.write().await;
                        let names = mgr.start_combat(room_id.clone(), combatants);

                        // Queue the player's first attack
                        mgr.queue_action(
                            &room_id,
                            CombatantId::Player(character_id.clone()),
                            CombatAction::Attack {
                                target: CombatantId::Monster(monster_id),
                            },
                        );
                        names
                    }; // mgr dropped here

                    debug!(combatants = ?names, "combat started");
                    self.send_combat(CombatServerMsg::CombatStart { combatants: names })
                        .await?;
                }
            }

            protocol::combat::ClientMsg::Flee => {
                let in_combat = {
                    let mut mgr = self.state.combat_manager.write().await;
                    if let Some(combat_room) = mgr.find_combat_for_player(&character_id) {
                        mgr.queue_action(
                            &combat_room,
                            CombatantId::Player(character_id.clone()),
                            CombatAction::Flee,
                        );
                        true
                    } else {
                        false
                    }
                }; // mgr dropped here

                if !in_combat {
                    self.send_combat(CombatServerMsg::ActionFail {
                        reason: "You are not in combat.".to_string(),
                    })
                    .await?;
                }
            }

            protocol::combat::ClientMsg::UseAbility { ability_name } => {
                let character_id_clone = character_id.clone();
                // Find current combat and queue ability against current target
                let queued = {
                    let mut mgr = self.state.combat_manager.write().await;
                    if let Some(combat_room) = mgr.find_combat_for_player(&character_id) {
                        // Find the first monster target in combat
                        let target = mgr.combats.get(&combat_room).and_then(|c| {
                            // Use last target or first monster
                            c.last_targets.get(&crate::combat::types::CombatantId::Player(character_id_clone.clone()))
                                .cloned()
                                .or_else(|| {
                                    c.combatants.iter().find_map(|cb| {
                                        if let crate::combat::types::CombatantId::Monster(id) = &cb.id {
                                            Some(crate::combat::types::CombatantId::Monster(id.clone()))
                                        } else { None }
                                    })
                                })
                        });
                        if let Some(target) = target {
                            mgr.queue_action(
                                &combat_room,
                                crate::combat::types::CombatantId::Player(character_id_clone),
                                crate::combat::types::CombatAction::UseAbility { ability_name: ability_name.clone(), target },
                            );
                            true
                        } else { false }
                    } else { false }
                };

                if !queued {
                    self.send_combat(CombatServerMsg::ActionFail {
                        reason: "You must be in combat to use abilities.".to_string(),
                    })
                    .await?;
                }
            }
        }
        Ok(())
    }

    /// Load a character's DEX score from the database.
    async fn load_character_dex(&self, character_id: &str) -> u8 {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT dex_score FROM characters WHERE id = ?")
                .bind(character_id)
                .fetch_optional(&self.state.db)
                .await
                .unwrap_or(None);
        row.map(|(d,)| d as u8).unwrap_or(10)
    }

    /// Send current vitals to the client.
    async fn send_vitals(&mut self, character_id: &str) -> anyhow::Result<()> {
        let row: Option<(i32, i32, i32, i32, i32, i32, i32, i64)> = sqlx::query_as(
            "SELECT hp, max_hp, mana, max_mana, stamina, max_stamina, xp, level FROM characters WHERE id = ?"
        )
        .bind(character_id)
        .fetch_optional(&self.state.db)
        .await?;

        if let Some((hp, max_hp, mana, max_mana, stamina, max_stamina, xp, level)) = row {
            self.send_combat(CombatServerMsg::Vitals {
                hp,
                max_hp,
                mana,
                max_mana,
                stamina,
                max_stamina,
                xp,
                level: level as i32,
            })
            .await?;
        }
        Ok(())
    }

    /// Check for aggressive monsters when entering a room and auto-start combat.
    async fn check_aggro(&mut self, room_id: &crate::world::types::RoomId) {
        let character_id = match &self.active_character_id {
            Some(id) => id.clone(),
            None => return,
        };
        let character_name = self
            .active_character_name
            .clone()
            .unwrap_or_else(|| character_id.clone());

        // Check for aggressive monsters in this room
        let aggressive_monster = {
            let monsters = self.state.active_monsters.read().await;
            if let Some(room_monsters) = monsters.get(room_id) {
                // Find first alive aggressive monster that's not already in combat
                let templates = &self.state.monster_templates;
                room_monsters
                    .iter()
                    .find(|m| {
                        m.is_alive()
                            && templates
                                .get(&m.template_id)
                                .map(|t| t.is_aggressive())
                                .unwrap_or(false)
                    })
                    .map(|m| (m.id.clone(), m.name.clone()))
            } else {
                None
            }
        };

        if let Some((monster_id, monster_name)) = aggressive_monster {
            // Don't start combat if one is already active in this room
            let already_in_combat = {
                let mgr = self.state.combat_manager.read().await;
                mgr.has_combat(room_id)
            };

            if !already_in_combat {
                use crate::combat::engine::roll_initiative;
                use crate::combat::types::*;

                let dex_score = self.load_character_dex(&character_id).await;
                let dex_mod = crate::character::types::ability_modifier(dex_score);

                let monster_id_for_action = monster_id.clone();
                let combatants = vec![
                    CombatantInfo {
                        id: CombatantId::Player(character_id.clone()),
                        name: character_name,
                        initiative: roll_initiative(dex_mod),
                    },
                    CombatantInfo {
                        id: CombatantId::Monster(monster_id),
                        name: monster_name,
                        initiative: roll_initiative(0),
                    },
                ];

                let names = {
                    let mut mgr = self.state.combat_manager.write().await;
                    let names = mgr.start_combat(room_id.clone(), combatants);

                    // Auto-queue the player's counterattack against the aggro monster
                    mgr.queue_action(
                        room_id,
                        CombatantId::Player(character_id.clone()),
                        CombatAction::Attack {
                            target: CombatantId::Monster(monster_id_for_action),
                        },
                    );

                    names
                }; // mgr dropped here

                // Send CombatStart to player
                if let Err(e) = self
                    .send_combat(CombatServerMsg::CombatStart { combatants: names })
                    .await
                {
                    warn!(error = %e, "failed to send aggro CombatStart");
                }
            }
        }
    }

    /// Graceful cleanup on connection drop: delete the active session and remove character from world.
    ///
    /// Satisfies AUTH-08: disconnecting without explicit logout still invalidates the session.
    async fn cleanup(&mut self) {
        // Remove character from in-memory world
        if let Some(char_id) = self.active_character_id.take() {
            let mut w = self.state.world.write().await;
            w.player_positions.remove(&char_id);
            w.player_names.remove(&char_id);
        }
        self.active_character_name = None;

        if let Some(token) = self.session_token.take() {
            if let Err(e) = delete_session(&self.state.db, &token).await {
                warn!(error = %e, "delete_session failed during cleanup");
            }
            self.account_id = None;
        }
        self.room_receiver = None;
    }

    /// Ensure the character has a position in the world. If not, place them at the default spawn.
    ///
    /// The spawn room is determined by (in priority order):
    /// 1. Persisted position in `character_positions` table
    /// 2. `world.default_spawn` — set at runtime by test helpers
    /// 3. `DEFAULT_SPAWN_ROOM` constant — the production default
    async fn ensure_character_in_world(&self, character_id: &str) {
        // Check for persisted position first
        let persisted_room: Option<(String,)> = sqlx::query_as(
            "SELECT room_id FROM character_positions WHERE character_id = ?"
        )
        .bind(character_id)
        .fetch_optional(&self.state.db)
        .await
        .unwrap_or(None);

        if let Some((room_id,)) = persisted_room {
            let mut w = self.state.world.write().await;
            w.player_positions
                .insert(character_id.to_string(), RoomId(room_id));
            return;
        }

        // No persisted position — use default spawn
        let spawn_override = {
            let w = self.state.world.read().await;
            w.default_spawn.clone()
        };

        let spawn = spawn_override
            .unwrap_or_else(|| RoomId(DEFAULT_SPAWN_ROOM.to_string()));

        // Insert into in-memory world
        {
            let mut w = self.state.world.write().await;
            w.player_positions
                .insert(character_id.to_string(), spawn.clone());
        }

        // Persist to SQLite
        if let Err(e) = sqlx::query(
            "INSERT OR IGNORE INTO character_positions (character_id, room_id, updated_at) VALUES (?, ?, unixepoch())"
        )
        .bind(character_id)
        .bind(&spawn.0)
        .execute(&self.state.db)
        .await
        {
            warn!(error = %e, "failed to persist initial character position");
        }
    }

    /// Subscribe the actor's room_receiver to the character's current room channel.
    async fn subscribe_to_current_room(&mut self, character_id: &str) {
        let room_id = {
            let w = self.state.world.read().await;
            w.player_positions.get(character_id).cloned()
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
