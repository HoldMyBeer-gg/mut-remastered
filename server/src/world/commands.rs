//! World command handlers — Look, Move, Examine, Interact.
//!
//! All reads come from the in-memory World (D-05: no DB reads on hot path).
//! Trigger evaluation is generic (D-11: no hardcoded room logic).
//! SQLite writes happen after write locks are dropped.

use std::collections::HashMap;
use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::{RwLock, broadcast};
use tracing::warn;

use crate::world::types::{Direction, RoomId, TriggerEffect, World, WorldEvent};
use protocol::world::ServerMsg;

/// Return the RoomDescription for the player's current room.
///
/// - Merges static exits with revealed extra_exits.
/// - Includes tutorial hints only when `tutorial_complete == false`.
/// - Lists other players present in the room.
pub async fn handle_look(
    world: &Arc<RwLock<World>>,
    account_id: &str,
    tutorial_complete: bool,
) -> ServerMsg {
    let w = world.read().await;

    let room_id = match w.player_positions.get(account_id) {
        Some(id) => id.clone(),
        None => {
            return ServerMsg::ExamineResult {
                text: "You are nowhere. This should not happen — please reconnect.".to_string(),
            };
        }
    };

    let room_def = match w.rooms.get(&room_id) {
        Some(r) => r,
        None => {
            return ServerMsg::ExamineResult {
                text: format!("Room '{}' not found in world data.", room_id),
            };
        }
    };

    // Merge static exits with dynamically revealed extra exits
    let room_state = w.room_states.get(&room_id);
    let mut exits: Vec<String> = room_def.exits.keys().cloned().collect();
    if let Some(state) = room_state {
        for key in state.extra_exits.keys() {
            if !exits.contains(key) {
                exits.push(key.clone());
            }
        }
    }
    exits.sort();

    // Other players in this room (not self)
    let players_here: Vec<String> = w
        .player_positions
        .iter()
        .filter(|(id, pos)| id.as_str() != account_id && **pos == room_id)
        .map(|(id, _)| id.clone())
        .collect();

    // Tutorial hints — only shown before tutorial is complete
    let hints = if !tutorial_complete {
        room_def.hints.clone().unwrap_or_default()
    } else {
        vec![]
    };

    ServerMsg::RoomDescription {
        room_id: room_id.0.clone(),
        name: room_def.name.clone(),
        description: room_def.description.clone(),
        exits,
        hints,
        players_here,
    }
}

/// Move the player in the given direction.
///
/// Returns (response, Option<auto-look RoomDescription>, Option<new_room_id>).
/// On success: MoveOk + new room description + new RoomId so actor can re-subscribe.
/// On failure: MoveFail + None + None.
pub async fn handle_move(
    world: &Arc<RwLock<World>>,
    room_channels: &Arc<RwLock<HashMap<RoomId, broadcast::Sender<WorldEvent>>>>,
    db: &SqlitePool,
    account_id: &str,
    direction_str: &str,
    tutorial_complete: bool,
) -> (ServerMsg, Option<ServerMsg>, Option<RoomId>) {
    // Parse direction
    let direction = match Direction::from_str(direction_str) {
        Some(d) => d,
        None => {
            return (
                ServerMsg::MoveFail {
                    reason: format!("Unknown direction '{direction_str}'."),
                },
                None,
                None,
            );
        }
    };

    let exit_key = direction.as_exit_key();

    // Read phase: validate current room and exit
    let (from_room_id, to_room_id_str) = {
        let w = world.read().await;

        let from_room_id = match w.player_positions.get(account_id) {
            Some(id) => id.clone(),
            None => {
                return (
                    ServerMsg::MoveFail {
                        reason: "You are not in the world yet.".to_string(),
                    },
                    None,
                    None,
                );
            }
        };

        let room_def = match w.rooms.get(&from_room_id) {
            Some(r) => r,
            None => {
                return (
                    ServerMsg::MoveFail {
                        reason: "Your current room is missing from world data.".to_string(),
                    },
                    None,
                    None,
                );
            }
        };

        // Check static exits first, then revealed extra exits
        let target = if let Some(target) = room_def.exits.get(exit_key) {
            target.clone()
        } else if let Some(state) = w.room_states.get(&from_room_id) {
            if let Some(target_id) = state.extra_exits.get(exit_key) {
                target_id.0.clone()
            } else {
                return (
                    ServerMsg::MoveFail {
                        reason: format!("There is no exit to the {}.", exit_key),
                    },
                    None,
                    None,
                );
            }
        } else {
            return (
                ServerMsg::MoveFail {
                    reason: format!("There is no exit to the {}.", exit_key),
                },
                None,
                None,
            );
        };

        (from_room_id, target)
    };

    let to_room_id = RoomId(to_room_id_str);

    // Write phase: update player position in memory
    {
        let mut w = world.write().await;
        w.player_positions
            .insert(account_id.to_string(), to_room_id.clone());
    }

    // Persist position to SQLite (after lock is dropped)
    if let Err(e) = sqlx::query(
        "INSERT OR REPLACE INTO player_positions (account_id, room_id, updated_at) VALUES (?, ?, unixepoch())"
    )
    .bind(account_id)
    .bind(&to_room_id.0)
    .execute(db)
    .await
    {
        warn!(error = %e, "failed to persist player position");
    }

    // Broadcast departure to old room
    broadcast_to_room(
        room_channels,
        &from_room_id,
        WorldEvent {
            message: format!("{} left to the {}.", account_id, exit_key),
        },
    )
    .await;

    // Broadcast arrival to new room
    broadcast_to_room(
        room_channels,
        &to_room_id,
        WorldEvent {
            message: format!("{} arrived from the {}.", account_id, direction.opposite()),
        },
    )
    .await;

    // Auto-look at the new room
    let room_desc = handle_look(world, account_id, tutorial_complete).await;

    (
        ServerMsg::MoveOk {
            from_room: from_room_id.0.clone(),
            to_room: to_room_id.0.clone(),
        },
        Some(room_desc),
        Some(to_room_id),
    )
}

/// Examine a target in the current room, returning lore or "nothing of note".
pub async fn handle_examine(
    world: &Arc<RwLock<World>>,
    account_id: &str,
    target: &str,
) -> ServerMsg {
    let w = world.read().await;

    let room_id = match w.player_positions.get(account_id) {
        Some(id) => id.clone(),
        None => {
            return ServerMsg::ExamineResult {
                text: "You are nowhere.".to_string(),
            };
        }
    };

    let room_def = match w.rooms.get(&room_id) {
        Some(r) => r,
        None => {
            return ServerMsg::ExamineResult {
                text: "You find nothing of note.".to_string(),
            };
        }
    };

    // If target is empty or refers to the room itself, return room lore
    let target_lower = target.to_lowercase();
    if target_lower.is_empty() || target_lower == "room" || target_lower == "here" {
        return ServerMsg::ExamineResult {
            text: room_def
                .lore
                .clone()
                .unwrap_or_else(|| "You find nothing of note.".to_string()),
        };
    }

    // Otherwise check if target keyword appears in lore context
    if let Some(ref lore) = room_def.lore {
        if lore.to_lowercase().contains(&target_lower) || room_def.name.to_lowercase().contains(&target_lower) {
            return ServerMsg::ExamineResult { text: lore.clone() };
        }
    }

    ServerMsg::ExamineResult {
        text: "You find nothing of note.".to_string(),
    }
}

/// Evaluate triggers matching `command` in the player's current room.
///
/// Returns a Vec of ServerMsg to send to the player.
/// Broadcasts are sent to room_channels internally.
/// Returns a sentinel `SetTutorialComplete` as a special WorldEvent for the actor to detect.
pub async fn handle_interact(
    world: &Arc<RwLock<World>>,
    room_channels: &Arc<RwLock<HashMap<RoomId, broadcast::Sender<WorldEvent>>>>,
    db: &SqlitePool,
    account_id: &str,
    command: &str,
) -> (Vec<ServerMsg>, bool) {
    let command_lower = command.to_lowercase();

    // Collect data needed under write lock
    let (room_id, matching_trigger) = {
        let w = world.read().await;

        let room_id = match w.player_positions.get(account_id) {
            Some(id) => id.clone(),
            None => {
                return (
                    vec![ServerMsg::InteractResult {
                        text: "You are nowhere.".to_string(),
                    }],
                    false,
                );
            }
        };

        let room_def = match w.rooms.get(&room_id) {
            Some(r) => r,
            None => {
                return (
                    vec![ServerMsg::InteractResult {
                        text: "Nothing happens.".to_string(),
                    }],
                    false,
                );
            }
        };

        let triggers = match &room_def.triggers {
            Some(t) => t.clone(),
            None => {
                return (
                    vec![ServerMsg::InteractResult {
                        text: "Nothing happens.".to_string(),
                    }],
                    false,
                );
            }
        };

        // Find first matching trigger (command matches and condition passes)
        let room_state = w.room_states.get(&room_id);
        let matched = triggers.into_iter().find(|trigger| {
            if trigger.command.to_lowercase() != command_lower {
                return false;
            }
            // Check condition if present.
            // When no room state exists (or the key is absent), treat the stored
            // value as "false" — this is the conventional initial state for boolean
            // trigger conditions (e.g., `lever_state = "false"` matches a lever
            // that has never been pulled).
            if let Some(ref cond) = trigger.condition {
                let current_value = room_state
                    .and_then(|s| s.kv.get(&cond.key))
                    .map(|v| v.as_str())
                    .unwrap_or("false");
                current_value == cond.value.as_str()
            } else {
                true
            }
        });

        (room_id, matched)
    };

    let trigger = match matching_trigger {
        Some(t) => t,
        None => {
            return (
                vec![ServerMsg::InteractResult {
                    text: "Nothing happens.".to_string(),
                }],
                false,
            );
        }
    };

    // Apply effects — collect state mutations and messages
    let mut responses: Vec<ServerMsg> = Vec::new();
    let mut set_tutorial_complete = false;
    let mut state_mutations: Vec<(String, String)> = Vec::new();
    let mut exit_reveals: Vec<(String, RoomId)> = Vec::new();
    let mut broadcasts: Vec<String> = Vec::new();

    for effect in &trigger.effects {
        match effect {
            TriggerEffect::Message { text } => {
                responses.push(ServerMsg::InteractResult { text: text.clone() });
            }
            TriggerEffect::Broadcast { text } => {
                let msg = text.replace("{player}", account_id);
                broadcasts.push(msg);
            }
            TriggerEffect::SetState { key, value } => {
                state_mutations.push((key.clone(), value.clone()));
            }
            TriggerEffect::RevealExit { direction, target } => {
                exit_reveals.push((direction.clone(), RoomId(target.clone())));
            }
            TriggerEffect::SetTutorialComplete => {
                set_tutorial_complete = true;
            }
        }
    }

    // Apply mutations under write lock
    if !state_mutations.is_empty() || !exit_reveals.is_empty() {
        let mut w = world.write().await;
        let state = w.room_states.entry(room_id.clone()).or_default();
        for (key, value) in &state_mutations {
            state.kv.insert(key.clone(), value.clone());
        }
        for (dir, target) in &exit_reveals {
            state.extra_exits.insert(dir.clone(), target.clone());
        }
    }

    // Persist state mutations to SQLite (after lock dropped)
    for (key, value) in &state_mutations {
        if let Err(e) = sqlx::query(
            "INSERT OR REPLACE INTO world_state (room_id, state_key, state_value, updated_at) VALUES (?, ?, ?, unixepoch())"
        )
        .bind(&room_id.0)
        .bind(key)
        .bind(value)
        .execute(db)
        .await
        {
            warn!(error = %e, "failed to persist world state");
        }
    }

    // Send broadcasts to room channel
    for msg in broadcasts {
        broadcast_to_room(
            room_channels,
            &room_id,
            WorldEvent { message: msg },
        )
        .await;
    }

    // Persist tutorial complete flag if set
    if set_tutorial_complete {
        if let Err(e) = sqlx::query(
            "INSERT OR IGNORE INTO account_flags (account_id, flag) VALUES (?, 'tutorial_complete')"
        )
        .bind(account_id)
        .execute(db)
        .await
        {
            warn!(error = %e, "failed to persist tutorial_complete flag");
        }
    }

    if responses.is_empty() {
        responses.push(ServerMsg::InteractResult {
            text: "Nothing happens.".to_string(),
        });
    }

    (responses, set_tutorial_complete)
}

/// Helper: send a WorldEvent to a room's broadcast channel, ignoring "no receivers" errors.
async fn broadcast_to_room(
    room_channels: &Arc<RwLock<HashMap<RoomId, broadcast::Sender<WorldEvent>>>>,
    room_id: &RoomId,
    event: WorldEvent,
) {
    let channels = room_channels.read().await;
    if let Some(sender) = channels.get(room_id) {
        // send() errors only if there are no receivers — that's fine
        let _ = sender.send(event);
    }
}
