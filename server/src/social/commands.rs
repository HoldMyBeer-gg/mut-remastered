//! Social command handlers: say, emote, whisper, gossip, look at, description, channel toggles.

use std::collections::HashMap;
use std::sync::Arc;

use sqlx::SqlitePool;
use tracing::warn;

use crate::inventory::types::ItemTemplate;
use crate::world::types::RoomId;
use protocol::world::{EquippedInfo, ServerMsg};

/// Handle the LookAt command: inspect another player in the same room.
pub async fn handle_look_at(
    db: &SqlitePool,
    world: &Arc<tokio::sync::RwLock<crate::world::types::World>>,
    character_id: &str,
    target: &str,
    item_templates: &Arc<HashMap<String, ItemTemplate>>,
) -> ServerMsg {
    // Find the target character in the same room
    let (room_id, target_char_id) = {
        let w = world.read().await;
        let my_room = match w.player_positions.get(character_id) {
            Some(r) => r.clone(),
            None => {
                return ServerMsg::WorldActionFail {
                    reason: "You are not in the world.".to_string(),
                };
            }
        };

        // Find a player in the same room whose name matches
        let target_lower = target.to_lowercase();
        let found = w
            .player_names
            .iter()
            .find(|(cid, name)| {
                *cid != character_id
                    && name.to_lowercase().contains(&target_lower)
                    && w.player_positions.get(*cid) == Some(&my_room)
            })
            .map(|(cid, _)| cid.clone());

        (my_room, found)
    };

    let target_char_id = match target_char_id {
        Some(id) => id,
        None => {
            return ServerMsg::WorldActionFail {
                reason: format!("No player '{}' found here.", target),
            };
        }
    };

    // Query character info
    let row: Option<(String, String, String, i64, String, String)> = sqlx::query_as(
        "SELECT name, race, class, level, description, bio FROM characters WHERE id = ?"
    )
    .bind(&target_char_id)
    .fetch_optional(db)
    .await
    .unwrap_or(None);

    let (name, race, class, level, description, bio) = match row {
        Some(r) => r,
        None => {
            return ServerMsg::WorldActionFail {
                reason: "Character not found.".to_string(),
            };
        }
    };

    // Get equipped items
    let equipped_rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT id, template_id, slot FROM items WHERE character_id = ? AND slot IS NOT NULL"
    )
    .bind(&target_char_id)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    let equipped: Vec<EquippedInfo> = equipped_rows
        .into_iter()
        .map(|(id, template_id, slot)| {
            let item_name = item_templates
                .get(&template_id)
                .map(|t| t.name.clone())
                .unwrap_or_else(|| template_id.clone());
            EquippedInfo {
                id,
                template_id,
                name: item_name,
                slot,
            }
        })
        .collect();

    ServerMsg::LookAtResult {
        name,
        race,
        class,
        level: level as i32,
        description,
        bio,
        equipped,
    }
}

/// Handle the SetDescription command: set visible character description (max 500 chars).
pub async fn handle_set_description(
    db: &SqlitePool,
    character_id: &str,
    text: &str,
) -> ServerMsg {
    if text.len() > 500 {
        return ServerMsg::WorldActionFail {
            reason: "Description must be 500 characters or fewer.".to_string(),
        };
    }

    if let Err(e) = sqlx::query("UPDATE characters SET description = ? WHERE id = ?")
        .bind(text)
        .bind(character_id)
        .execute(db)
        .await
    {
        warn!(error = %e, "failed to update description");
        return ServerMsg::WorldActionFail {
            reason: "Failed to update description.".to_string(),
        };
    }

    ServerMsg::DescriptionOk
}

/// Handle the ToggleChannel command.
pub async fn handle_toggle_channel(
    db: &SqlitePool,
    character_id: &str,
    channel: &str,
) -> ServerMsg {
    let valid_channels = ["say", "gossip", "emote", "whisper"];
    if !valid_channels.contains(&channel) {
        return ServerMsg::WorldActionFail {
            reason: format!(
                "Unknown channel '{}'. Valid: {}",
                channel,
                valid_channels.join(", ")
            ),
        };
    }

    // Check current state
    let current: Option<(bool,)> = sqlx::query_as(
        "SELECT enabled FROM channel_toggles WHERE character_id = ? AND channel = ?"
    )
    .bind(character_id)
    .bind(channel)
    .fetch_optional(db)
    .await
    .unwrap_or(None);

    let new_enabled = match current {
        Some((true,)) => false,
        Some((false,)) => true,
        None => false, // Default is enabled; first toggle turns it off
    };

    let _ = sqlx::query(
        "INSERT OR REPLACE INTO channel_toggles (character_id, channel, enabled) VALUES (?, ?, ?)"
    )
    .bind(character_id)
    .bind(channel)
    .bind(new_enabled)
    .execute(db)
    .await;

    ServerMsg::ChannelToggled {
        channel: channel.to_string(),
        enabled: new_enabled,
    }
}

/// Check if a channel is enabled for a character (default: true).
pub async fn is_channel_enabled(
    db: &SqlitePool,
    character_id: &str,
    channel: &str,
) -> bool {
    let row: Option<(bool,)> = sqlx::query_as(
        "SELECT enabled FROM channel_toggles WHERE character_id = ? AND channel = ?"
    )
    .bind(character_id)
    .bind(channel)
    .fetch_optional(db)
    .await
    .unwrap_or(None);

    match row {
        Some((enabled,)) => enabled,
        None => true, // Default enabled
    }
}
