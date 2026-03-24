//! Item template types and body slot definitions.

use std::collections::HashMap;
use serde::Deserialize;

/// Top-level TOML structure for the items data file.
#[derive(Debug, Deserialize)]
pub struct ItemDataFile {
    pub items: Vec<ItemTemplate>,
}

/// Static item definition loaded from TOML. Never mutated after load.
#[derive(Debug, Clone, Deserialize)]
pub struct ItemTemplate {
    pub id: String,
    pub name: String,
    pub kind: String, // "weapon", "armor", "accessory", "junk"
    #[serde(default)]
    pub slot: Option<String>, // body slot this equips to (None for junk)
    pub description: String,
    // Weapon fields
    #[serde(default)]
    pub damage_dice: u32,
    #[serde(default)]
    pub damage_sides: u32,
    #[serde(default)]
    pub damage_bonus: i32,
    #[serde(default)]
    pub ability: Option<String>, // "str" or "dex" for weapon attack ability
    // Armor fields
    #[serde(default)]
    pub ac_bonus: i32,
    // Accessory fields
    #[serde(default)]
    pub stat_bonus_ability: Option<String>,
    #[serde(default)]
    pub stat_bonus_value: i32,
}

impl ItemTemplate {
    pub fn is_equippable(&self) -> bool {
        self.slot.is_some()
    }
}

/// Valid body slots for equipment.
pub const BODY_SLOTS: &[&str] = &[
    "head", "neck", "body", "arms", "hands", "legs", "feet",
    "ring_1", "ring_2", "weapon", "offhand",
];

/// Check if a slot name is valid.
pub fn is_valid_slot(slot: &str) -> bool {
    BODY_SLOTS.contains(&slot)
}

/// Load item templates from a TOML file.
pub fn load_item_templates(
    data_path: &std::path::Path,
) -> anyhow::Result<HashMap<String, ItemTemplate>> {
    let content = std::fs::read_to_string(data_path)?;
    let data: ItemDataFile = toml::from_str(&content)?;
    let mut templates = HashMap::new();
    for item in data.items {
        templates.insert(item.id.clone(), item);
    }
    Ok(templates)
}
