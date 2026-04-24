//! Inventory command handlers: get, drop, equip, unequip, inventory, stats, bio.

use std::collections::HashMap;
use std::sync::Arc;

use sqlx::SqlitePool;
use tracing::warn;

use crate::character::types::ability_modifier;
use crate::inventory::types::ItemTemplate;
use protocol::world::{EquippedInfo, ItemInfo, ServerMsg};

/// Handle the Inventory command: list carried and equipped items.
pub async fn handle_inventory(
    db: &SqlitePool,
    character_id: &str,
    item_templates: &Arc<HashMap<String, ItemTemplate>>,
) -> ServerMsg {
    // Carried items (slot IS NULL)
    let carried: Vec<(String, String)> =
        sqlx::query_as("SELECT id, template_id FROM items WHERE character_id = ? AND slot IS NULL")
            .bind(character_id)
            .fetch_all(db)
            .await
            .unwrap_or_default();

    let items: Vec<ItemInfo> = carried
        .into_iter()
        .map(|(id, template_id)| {
            let name = item_templates
                .get(&template_id)
                .map(|t| t.name.clone())
                .unwrap_or_else(|| template_id.clone());
            ItemInfo {
                id,
                template_id,
                name,
            }
        })
        .collect();

    // Equipped items (slot IS NOT NULL)
    let equipped_rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT id, template_id, slot FROM items WHERE character_id = ? AND slot IS NOT NULL",
    )
    .bind(character_id)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    let equipped: Vec<EquippedInfo> = equipped_rows
        .into_iter()
        .map(|(id, template_id, slot)| {
            let name = item_templates
                .get(&template_id)
                .map(|t| t.name.clone())
                .unwrap_or_else(|| template_id.clone());
            EquippedInfo {
                id,
                template_id,
                name,
                slot,
            }
        })
        .collect();

    // Gold
    let gold: i32 = sqlx::query_scalar("SELECT gold FROM characters WHERE id = ?")
        .bind(character_id)
        .fetch_one(db)
        .await
        .unwrap_or(0);

    ServerMsg::InventoryList {
        items,
        equipped,
        gold,
    }
}

/// Handle the GetItem command: pick up an item from the room floor.
pub async fn handle_get_item(
    db: &SqlitePool,
    character_id: &str,
    room_id: &str,
    target: &str,
    item_templates: &Arc<HashMap<String, ItemTemplate>>,
) -> ServerMsg {
    // Find matching item on room floor
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT id, template_id FROM room_items WHERE room_id = ?")
            .bind(room_id)
            .fetch_all(db)
            .await
            .unwrap_or_default()
            .into_iter()
            .find(|(_, tid)| {
                item_templates
                    .get(tid)
                    .map(|t| t.name.to_lowercase().contains(&target.to_lowercase()))
                    .unwrap_or(false)
            });

    let (item_id, template_id) = match row {
        Some(r) => r,
        None => {
            return ServerMsg::WorldActionFail {
                reason: format!("No item '{}' found here.", target),
            };
        }
    };

    let item_name = item_templates
        .get(&template_id)
        .map(|t| t.name.clone())
        .unwrap_or_else(|| template_id.clone());

    // Move item: delete from room_items, insert into items
    let new_item_id = uuid::Uuid::new_v4().to_string();
    if let Err(e) = sqlx::query("DELETE FROM room_items WHERE id = ?")
        .bind(&item_id)
        .execute(db)
        .await
    {
        warn!(error = %e, "failed to remove room item");
        return ServerMsg::WorldActionFail {
            reason: "Failed to pick up item.".to_string(),
        };
    }

    if let Err(e) =
        sqlx::query("INSERT INTO items (id, character_id, template_id) VALUES (?, ?, ?)")
            .bind(&new_item_id)
            .bind(character_id)
            .bind(&template_id)
            .execute(db)
            .await
    {
        warn!(error = %e, "failed to add item to inventory");
        return ServerMsg::WorldActionFail {
            reason: "Failed to pick up item.".to_string(),
        };
    }

    ServerMsg::GetItemOk { item_name }
}

/// Handle the DropItem command: drop an item from inventory to the room floor.
pub async fn handle_drop_item(
    db: &SqlitePool,
    character_id: &str,
    room_id: &str,
    target: &str,
    item_templates: &Arc<HashMap<String, ItemTemplate>>,
) -> ServerMsg {
    // Find matching item in inventory (not equipped)
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT id, template_id FROM items WHERE character_id = ? AND slot IS NULL")
            .bind(character_id)
            .fetch_all(db)
            .await
            .unwrap_or_default();

    let row = rows.into_iter().find(|(_, tid)| {
        item_templates
            .get(tid)
            .map(|t| t.name.to_lowercase().contains(&target.to_lowercase()))
            .unwrap_or(false)
    });

    let (item_id, template_id) = match row {
        Some(r) => r,
        None => {
            return ServerMsg::WorldActionFail {
                reason: format!("You don't have '{}'.", target),
            };
        }
    };

    let item_name = item_templates
        .get(&template_id)
        .map(|t| t.name.clone())
        .unwrap_or_else(|| template_id.clone());

    // Move: delete from items, insert into room_items
    let _ = sqlx::query("DELETE FROM items WHERE id = ?")
        .bind(&item_id)
        .execute(db)
        .await;

    let room_item_id = uuid::Uuid::new_v4().to_string();
    let _ = sqlx::query("INSERT INTO room_items (id, room_id, template_id) VALUES (?, ?, ?)")
        .bind(&room_item_id)
        .bind(room_id)
        .bind(&template_id)
        .execute(db)
        .await;

    ServerMsg::DropItemOk { item_name }
}

/// Handle the Equip command: equip an inventory item to its body slot.
pub async fn handle_equip(
    db: &SqlitePool,
    character_id: &str,
    item_name_query: &str,
    item_templates: &Arc<HashMap<String, ItemTemplate>>,
) -> ServerMsg {
    // Find matching item in inventory (not equipped)
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT id, template_id FROM items WHERE character_id = ? AND slot IS NULL")
            .bind(character_id)
            .fetch_all(db)
            .await
            .unwrap_or_default();

    let row = rows.into_iter().find(|(_, tid)| {
        item_templates
            .get(tid)
            .map(|t| {
                t.name
                    .to_lowercase()
                    .contains(&item_name_query.to_lowercase())
            })
            .unwrap_or(false)
    });

    let (item_id, template_id) = match row {
        Some(r) => r,
        None => {
            return ServerMsg::WorldActionFail {
                reason: format!("You don't have '{}'.", item_name_query),
            };
        }
    };

    let template = match item_templates.get(&template_id) {
        Some(t) => t,
        None => {
            return ServerMsg::WorldActionFail {
                reason: "Unknown item type.".to_string(),
            };
        }
    };

    let target_slot = match &template.slot {
        Some(s) => {
            // Handle ring slot: use ring_1 if empty, else ring_2
            if s == "ring" {
                let ring1_occupied: bool = sqlx::query_scalar(
                    "SELECT COUNT(*) > 0 FROM items WHERE character_id = ? AND slot = 'ring_1'",
                )
                .bind(character_id)
                .fetch_one(db)
                .await
                .unwrap_or(false);
                if ring1_occupied {
                    "ring_2".to_string()
                } else {
                    "ring_1".to_string()
                }
            } else {
                s.clone()
            }
        }
        None => {
            return ServerMsg::WorldActionFail {
                reason: format!("{} cannot be equipped.", template.name),
            };
        }
    };

    // Unequip existing item in that slot (swap to inventory)
    let _ = sqlx::query("UPDATE items SET slot = NULL WHERE character_id = ? AND slot = ?")
        .bind(character_id)
        .bind(&target_slot)
        .execute(db)
        .await;

    // Equip the new item
    let _ = sqlx::query("UPDATE items SET slot = ? WHERE id = ?")
        .bind(&target_slot)
        .bind(&item_id)
        .execute(db)
        .await;

    ServerMsg::EquipOk {
        item_name: template.name.clone(),
        slot: target_slot,
    }
}

/// Handle the Unequip command: move an equipped item back to inventory.
pub async fn handle_unequip(
    db: &SqlitePool,
    character_id: &str,
    slot: &str,
    item_templates: &Arc<HashMap<String, ItemTemplate>>,
) -> ServerMsg {
    // Find item in that slot
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT id, template_id FROM items WHERE character_id = ? AND slot = ?")
            .bind(character_id)
            .bind(slot)
            .fetch_optional(db)
            .await
            .unwrap_or(None);

    let (item_id, template_id) = match row {
        Some(r) => r,
        None => {
            return ServerMsg::WorldActionFail {
                reason: format!("Nothing equipped in slot '{}'.", slot),
            };
        }
    };

    let item_name = item_templates
        .get(&template_id)
        .map(|t| t.name.clone())
        .unwrap_or_else(|| template_id.clone());

    // Move to inventory (set slot = NULL)
    let _ = sqlx::query("UPDATE items SET slot = NULL WHERE id = ?")
        .bind(&item_id)
        .execute(db)
        .await;

    ServerMsg::UnequipOk {
        slot: slot.to_string(),
        item_name,
    }
}

/// Handle the Stats command: display character stats with equipment bonuses.
pub async fn handle_stats(
    db: &SqlitePool,
    character_id: &str,
    item_templates: &Arc<HashMap<String, ItemTemplate>>,
) -> ServerMsg {
    // Split into two queries to stay within sqlx's 16-tuple limit
    let row1: Option<(String, String, String, i64, i32, i32, i32, i32, i32, i32, i32)> =
        sqlx::query_as(
            "SELECT name, race, class, level, xp, hp, max_hp, mana, max_mana, stamina, max_stamina FROM characters WHERE id = ?"
        )
        .bind(character_id)
        .fetch_optional(db)
        .await
        .unwrap_or(None);

    let (name, race, class, level, xp, hp, max_hp, mana, max_mana, stamina, max_stamina) =
        match row1 {
            Some(r) => r,
            None => {
                return ServerMsg::WorldActionFail {
                    reason: "Character not found.".to_string(),
                };
            }
        };

    let row2: Option<(i32, i32, i32, i32, i32, i32, String)> =
        sqlx::query_as(
            "SELECT str_score, dex_score, con_score, int_score, wis_score, cha_score, bio FROM characters WHERE id = ?"
        )
        .bind(character_id)
        .fetch_optional(db)
        .await
        .unwrap_or(None);

    let (str_s, dex_s, con_s, int_s, wis_s, cha_s, bio) =
        row2.unwrap_or((10, 10, 10, 10, 10, 10, String::new()));

    // Calculate AC with equipment bonuses
    let base_ac = 10 + ability_modifier(dex_s as u8);
    let ac_bonus = calculate_ac_bonus(db, character_id, item_templates).await;

    ServerMsg::StatsResult {
        name,
        race,
        class,
        level: level as i32,
        xp,
        hp,
        max_hp,
        mana,
        max_mana,
        stamina,
        max_stamina,
        str_score: str_s,
        dex_score: dex_s,
        con_score: con_s,
        int_score: int_s,
        wis_score: wis_s,
        cha_score: cha_s,
        ac: base_ac + ac_bonus,
        bio,
    }
}

/// Handle the Bio command: set character biography (max 500 chars).
pub async fn handle_bio(db: &SqlitePool, character_id: &str, text: &str) -> ServerMsg {
    if text.len() > 500 {
        return ServerMsg::WorldActionFail {
            reason: "Biography must be 500 characters or fewer.".to_string(),
        };
    }

    if let Err(e) = sqlx::query("UPDATE characters SET bio = ? WHERE id = ?")
        .bind(text)
        .bind(character_id)
        .execute(db)
        .await
    {
        warn!(error = %e, "failed to update bio");
        return ServerMsg::WorldActionFail {
            reason: "Failed to update biography.".to_string(),
        };
    }

    ServerMsg::BioOk
}

/// Calculate total AC bonus from equipped armor.
pub async fn calculate_ac_bonus(
    db: &SqlitePool,
    character_id: &str,
    item_templates: &Arc<HashMap<String, ItemTemplate>>,
) -> i32 {
    let equipped: Vec<(String,)> =
        sqlx::query_as("SELECT template_id FROM items WHERE character_id = ? AND slot IS NOT NULL")
            .bind(character_id)
            .fetch_all(db)
            .await
            .unwrap_or_default();

    equipped
        .iter()
        .filter_map(|(tid,)| item_templates.get(tid))
        .map(|t| t.ac_bonus)
        .sum()
}
