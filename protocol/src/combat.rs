use serde::{Deserialize, Serialize};

/// Client → Server messages for combat actions.
///
/// Sent while in the world with an active character.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClientMsg {
    /// Attack a target by name (NPC monster name or partial match).
    Attack { target: String },
    /// Attempt to flee from combat (50% chance per attempt).
    Flee,
    /// Use a class ability by name (e.g., "power_strike", "heal", "arcane_blast").
    UseAbility { ability_name: String },
}

/// Server → Client messages for combat events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerMsg {
    /// Current vital statistics — pushed after any state-changing action.
    Vitals {
        hp: i32,
        max_hp: i32,
        mana: i32,
        max_mana: i32,
        stamina: i32,
        max_stamina: i32,
        xp: i32,
        level: i32,
    },
    /// Combat log entries for one round — dice roll results in plain language.
    CombatLog { entries: Vec<String> },
    /// Combat has started — lists all combatants by name.
    CombatStart { combatants: Vec<String> },
    /// Combat has ended — result describes outcome (victory, fled, death).
    CombatEnd { result: String },
    /// A generic failure for combat/inventory actions.
    ActionFail { reason: String },
}
