use serde::{Deserialize, Serialize};

/// Client → Server messages for character management.
///
/// Sent after login and before entering the world. The flow is:
/// 1. `CharacterList` — see existing characters
/// 2. `CharacterCreate` — make a new one (optional)
/// 3. `CharacterSelect` — choose one to play
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClientMsg {
    /// List all characters on the current account.
    CharacterList,
    /// Create a new character with the given attributes.
    CharacterCreate {
        name: String,
        race: String,
        class: String,
        gender: String,
        /// Base ability scores before racial bonuses: [STR, DEX, CON, INT, WIS, CHA].
        /// Must be valid point-buy (all 8-15, total cost = 27).
        ability_scores: [u8; 6],
        /// For races with flexible racial bonuses:
        /// - Human: two ability indices (0-5) that each get +1
        /// - Half-Elf: one ability index (0-5) that gets +1
        /// - Other races: ignored
        racial_bonus_choices: Vec<u8>,
    },
    /// Select an existing character to enter the world.
    CharacterSelect { character_id: String },
}

/// Server → Client messages for character management.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerMsg {
    CharacterListResult {
        characters: Vec<CharacterSummary>,
    },
    CharacterCreateOk {
        character_id: String,
        name: String,
    },
    CharacterCreateFail {
        reason: String,
    },
    CharacterSelected {
        character_id: String,
        name: String,
    },
    CharacterSelectFail {
        reason: String,
    },
}

/// Summary of a character shown in the selection list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CharacterSummary {
    pub id: String,
    pub name: String,
    pub race: String,
    pub class: String,
    pub level: u32,
}
