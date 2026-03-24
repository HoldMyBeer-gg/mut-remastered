use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClientMsg {
    /// Move in a direction. Accepts full names or aliases: "n", "north", etc.
    Move { direction: String },
    /// Re-display current room description
    Look,
    /// Examine an object/feature in the room (returns lore text or "nothing of note")
    Examine { target: String },
    /// Freeform trigger activation command (e.g., "pull lever", "read sign")
    Interact { command: String },
    /// List carried and equipped items
    Inventory,
    /// Pick up an item from the room floor
    GetItem { target: String },
    /// Drop an item from inventory to the room floor
    DropItem { target: String },
    /// Equip an inventory item to its appropriate body slot
    Equip { item_name: String },
    /// Unequip an item from a body slot back to inventory
    Unequip { slot: String },
    /// View own character stats (ability scores, AC, HP, etc.)
    Stats,
    /// Set character biography (max 500 characters)
    Bio { text: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerMsg {
    RoomDescription {
        room_id: String,
        name: String,
        description: String,
        exits: Vec<String>,
        hints: Vec<String>,
        players_here: Vec<String>,
    },
    MoveOk {
        from_room: String,
        to_room: String,
    },
    MoveFail {
        reason: String,
    },
    ExamineResult {
        text: String,
    },
    InteractResult {
        text: String,
    },
    WorldEvent {
        message: String,
    },
    /// Inventory listing: carried items + equipped items by slot.
    InventoryList {
        items: Vec<ItemInfo>,
        equipped: Vec<EquippedInfo>,
        gold: i32,
    },
    GetItemOk { item_name: String },
    DropItemOk { item_name: String },
    EquipOk { item_name: String, slot: String },
    UnequipOk { slot: String, item_name: String },
    /// Character stats display.
    StatsResult {
        name: String,
        race: String,
        class: String,
        level: i32,
        xp: i32,
        hp: i32,
        max_hp: i32,
        mana: i32,
        max_mana: i32,
        stamina: i32,
        max_stamina: i32,
        str_score: i32,
        dex_score: i32,
        con_score: i32,
        int_score: i32,
        wis_score: i32,
        cha_score: i32,
        ac: i32,
        bio: String,
    },
    BioOk,
    /// Generic failure for world/inventory actions.
    WorldActionFail { reason: String },
}

/// Info about an item in inventory (not equipped).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ItemInfo {
    pub id: String,
    pub template_id: String,
    pub name: String,
}

/// Info about an equipped item.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EquippedInfo {
    pub id: String,
    pub template_id: String,
    pub name: String,
    pub slot: String,
}
