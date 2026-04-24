//! Combat data types: monster templates, active monsters, combat instances, dice.

use std::collections::HashMap;

use serde::Deserialize;

use crate::world::types::RoomId;

// ── Monster Templates (loaded from TOML) ──────────────────────────────────

/// Top-level TOML structure for the monsters data file.
#[derive(Debug, Deserialize)]
pub struct MonsterDataFile {
    pub monsters: Vec<MonsterTemplate>,
}

/// Static monster definition loaded from TOML. Never mutated after load.
#[derive(Debug, Clone, Deserialize)]
pub struct MonsterTemplate {
    pub id: String,
    pub name: String,
    pub hp: i32,
    pub ac: i32,
    pub attack_bonus: i32,
    /// Number of damage dice (e.g., 1 for 1d6).
    pub damage_dice: u32,
    /// Sides per damage die (e.g., 6 for 1d6).
    pub damage_sides: u32,
    /// Flat bonus added to damage roll.
    #[serde(default)]
    pub damage_bonus: i32,
    pub xp_value: i32,
    /// "passive" or "aggressive"
    #[serde(default = "default_aggro")]
    pub aggro: String,
    #[serde(default)]
    pub loot: Vec<LootEntry>,
}

fn default_aggro() -> String {
    "passive".to_string()
}

impl MonsterTemplate {
    pub fn is_aggressive(&self) -> bool {
        self.aggro == "aggressive"
    }

    /// Format the damage as a string like "1d6+2".
    pub fn damage_string(&self) -> String {
        if self.damage_bonus > 0 {
            format!(
                "{}d{}+{}",
                self.damage_dice, self.damage_sides, self.damage_bonus
            )
        } else if self.damage_bonus < 0 {
            format!(
                "{}d{}{}",
                self.damage_dice, self.damage_sides, self.damage_bonus
            )
        } else {
            format!("{}d{}", self.damage_dice, self.damage_sides)
        }
    }
}

/// A single loot table entry. Either an item drop or a gold range.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum LootEntry {
    Item { item: String, chance: u32 },
    Gold { gold_min: i32, gold_max: i32 },
}

// ── Spawn Tables (in zone TOML) ──────────────────────────────────────────

/// Spawn entry defined per-room in zone TOML files.
#[derive(Debug, Clone, Deserialize)]
pub struct SpawnEntry {
    pub monster: String,
    pub count: u32,
    #[serde(default = "default_respawn")]
    pub respawn_secs: u64,
}

fn default_respawn() -> u64 {
    120
}

// ── Active Monsters (runtime) ────────────────────────────────────────────

/// A live monster instance in the world.
#[derive(Debug, Clone)]
pub struct ActiveMonster {
    pub id: String,
    pub template_id: String,
    pub name: String,
    pub room_id: RoomId,
    pub hp: i32,
    pub max_hp: i32,
    pub ac: i32,
    pub attack_bonus: i32,
    pub damage_dice: u32,
    pub damage_sides: u32,
    pub damage_bonus: i32,
    pub xp_value: i32,
}

impl ActiveMonster {
    pub fn from_template(template: &MonsterTemplate, room_id: &RoomId) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            template_id: template.id.clone(),
            name: template.name.clone(),
            room_id: room_id.clone(),
            hp: template.hp,
            max_hp: template.hp,
            ac: template.ac,
            attack_bonus: template.attack_bonus,
            damage_dice: template.damage_dice,
            damage_sides: template.damage_sides,
            damage_bonus: template.damage_bonus,
            xp_value: template.xp_value,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }

    pub fn damage_string(&self) -> String {
        if self.damage_bonus > 0 {
            format!(
                "{}d{}+{}",
                self.damage_dice, self.damage_sides, self.damage_bonus
            )
        } else if self.damage_bonus < 0 {
            format!(
                "{}d{}{}",
                self.damage_dice, self.damage_sides, self.damage_bonus
            )
        } else {
            format!("{}d{}", self.damage_dice, self.damage_sides)
        }
    }
}

// ── Combat Instance ──────────────────────────────────────────────────────

/// Identifies a combatant in a combat instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CombatantId {
    Player(String),  // character_id
    Monster(String), // active_monster.id
}

/// A single combatant's stats cached at combat start.
#[derive(Debug, Clone)]
pub struct CombatantInfo {
    pub id: CombatantId,
    pub name: String,
    pub initiative: i32,
}

/// An action queued for the current round.
#[derive(Debug, Clone)]
pub enum CombatAction {
    Attack {
        target: CombatantId,
    },
    Flee,
    UseAbility {
        ability_name: String,
        target: CombatantId,
    },
}

/// Result of processing one combat round.
#[derive(Debug)]
pub struct RoundResult {
    /// Combat log entries to broadcast to the room.
    pub log_entries: Vec<String>,
    /// Characters whose vitals changed (character_id -> new vitals).
    pub vitals_updates: Vec<(String, VitalsSnapshot)>,
    /// Characters who died this round.
    pub deaths: Vec<DeathEvent>,
    /// Monsters killed this round (for loot and XP).
    pub monster_kills: Vec<MonsterKill>,
    /// Characters who fled successfully.
    pub fled: Vec<String>,
    /// Whether combat has ended.
    pub combat_ended: bool,
    /// End result message (if combat ended).
    pub end_message: Option<String>,
}

/// Snapshot of a character's vitals for sending to the client.
#[derive(Debug, Clone)]
pub struct VitalsSnapshot {
    pub hp: i32,
    pub max_hp: i32,
    pub mana: i32,
    pub max_mana: i32,
    pub stamina: i32,
    pub max_stamina: i32,
    pub xp: i32,
    pub level: i32,
}

/// A player death event.
#[derive(Debug)]
pub struct DeathEvent {
    pub character_id: String,
    pub character_name: String,
}

/// A monster that was killed this round.
#[derive(Debug)]
pub struct MonsterKill {
    pub monster_id: String,
    pub template_id: String,
    pub room_id: RoomId,
    pub xp_value: i32,
    /// Character IDs of players who participated in the kill (for XP split).
    pub participants: Vec<String>,
}

/// Respawn timer for a dead monster.
#[derive(Debug)]
pub struct RespawnTimer {
    pub template_id: String,
    pub room_id: RoomId,
    pub respawn_at: tokio::time::Instant,
}

/// The full combat state for one room.
#[derive(Debug)]
pub struct CombatInstance {
    pub room_id: RoomId,
    /// All combatants in initiative order.
    pub combatants: Vec<CombatantInfo>,
    /// Queued actions for the current round. Key = combatant ID.
    pub queued_actions: HashMap<CombatantId, CombatAction>,
    /// Number of rounds elapsed.
    pub round: u32,
    /// Tracks each player's last attack target for auto-attack.
    pub last_targets: HashMap<CombatantId, CombatantId>,
    /// Ability cooldowns: (combatant_id, ability_name) → rounds remaining.
    pub cooldowns: HashMap<(CombatantId, String), u32>,
    /// Whether Rogue's sneak attack has been used this combat.
    pub sneak_attack_used: HashMap<CombatantId, bool>,
    /// Players involved (character_ids) — for XP distribution.
    pub player_participants: Vec<String>,
}

impl CombatInstance {
    pub fn new(room_id: RoomId, combatants: Vec<CombatantInfo>) -> Self {
        let player_participants: Vec<String> = combatants
            .iter()
            .filter_map(|c| match &c.id {
                CombatantId::Player(id) => Some(id.clone()),
                _ => None,
            })
            .collect();

        Self {
            room_id,
            combatants,
            queued_actions: HashMap::new(),
            round: 0,
            last_targets: HashMap::new(),
            cooldowns: HashMap::new(),
            sneak_attack_used: HashMap::new(),
            player_participants,
        }
    }

    /// Check if a combatant is in this combat.
    pub fn has_combatant(&self, id: &CombatantId) -> bool {
        self.combatants.iter().any(|c| c.id == *id)
    }

    /// Remove a combatant (on death or flee).
    pub fn remove_combatant(&mut self, id: &CombatantId) {
        self.combatants.retain(|c| c.id != *id);
        self.queued_actions.remove(id);
    }

    /// Check if any players remain in combat.
    pub fn has_players(&self) -> bool {
        self.combatants
            .iter()
            .any(|c| matches!(c.id, CombatantId::Player(_)))
    }

    /// Check if any monsters remain in combat.
    pub fn has_monsters(&self) -> bool {
        self.combatants
            .iter()
            .any(|c| matches!(c.id, CombatantId::Monster(_)))
    }
}
