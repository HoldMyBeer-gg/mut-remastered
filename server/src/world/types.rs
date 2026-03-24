use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Newtype wrapper for room identifiers (e.g., "starting_village:market_square").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoomId(pub String);

impl fmt::Display for RoomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Cardinal and vertical directions for room navigation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Direction {
    North,
    South,
    East,
    West,
    Up,
    Down,
}

impl Direction {
    /// Parse a direction from a string, accepting both full names and single-letter aliases.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "north" | "n" => Some(Direction::North),
            "south" | "s" => Some(Direction::South),
            "east"  | "e" => Some(Direction::East),
            "west"  | "w" => Some(Direction::West),
            "up"    | "u" => Some(Direction::Up),
            "down"  | "d" => Some(Direction::Down),
            _ => None,
        }
    }

    /// Return the canonical lowercase exit key used in TOML and exit HashMaps.
    pub fn as_exit_key(&self) -> &'static str {
        match self {
            Direction::North => "north",
            Direction::South => "south",
            Direction::East  => "east",
            Direction::West  => "west",
            Direction::Up    => "up",
            Direction::Down  => "down",
        }
    }

    /// Return the opposite direction label (used in arrival broadcast messages).
    pub fn opposite(&self) -> &'static str {
        match self {
            Direction::North => "south",
            Direction::South => "north",
            Direction::East  => "west",
            Direction::West  => "east",
            Direction::Up    => "below",
            Direction::Down  => "above",
        }
    }
}

/// Static definition of a room as loaded from a zone TOML file.
///
/// These are immutable after load — mutations go into `RoomState`.
#[derive(Debug, Clone, Deserialize)]
pub struct RoomDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub lore: Option<String>,
    pub hints: Option<Vec<String>>,
    /// Map from direction key (e.g., "north") to target room ID string.
    pub exits: HashMap<String, String>,
    pub triggers: Option<Vec<TriggerDef>>,
}

/// A data-driven trigger defined in TOML.
///
/// Activates when a player sends an Interact command matching `command`.
#[derive(Debug, Clone, Deserialize)]
pub struct TriggerDef {
    pub command: String,
    pub condition: Option<TriggerCondition>,
    pub effects: Vec<TriggerEffect>,
}

/// Optional pre-condition for a trigger — checks that a room state key equals a value.
#[derive(Debug, Clone, Deserialize)]
pub struct TriggerCondition {
    pub key: String,
    pub value: String,
}

/// Effects produced when a trigger fires.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TriggerEffect {
    /// Send a message only to the player who activated the trigger.
    Message { text: String },
    /// Broadcast a message to all players in the room.
    /// The `{player}` placeholder will be replaced with the acting player's username.
    Broadcast { text: String },
    /// Set a persistent key/value pair in the room's state.
    SetState { key: String, value: String },
    /// Reveal a new exit in the room (e.g., open a door).
    RevealExit { direction: String, target: String },
    /// Mark the player's tutorial as complete.
    SetTutorialComplete,
}

/// Mutable runtime state for a single room.
///
/// The base definition is in `RoomDef`; this struct holds state that changes
/// as players interact with the world. Persisted to SQLite and overlaid on load.
#[derive(Debug, Default, Clone)]
pub struct RoomState {
    /// Arbitrary key/value state set by trigger effects (e.g., "gate_lever" = "true").
    pub kv: HashMap<String, String>,
    /// Extra exits revealed by trigger effects (e.g., a secret door opened).
    pub extra_exits: HashMap<String, RoomId>,
}

/// The complete in-memory world state.
///
/// This is wrapped in `Arc<RwLock<World>>` in `AppState`. Read locks are used
/// for all query operations (look, move validity); write locks only on mutations
/// (player moves, trigger fires). SQLite is the persistence layer, not the query layer.
#[derive(Debug, Default)]
pub struct World {
    /// All room definitions keyed by room ID.
    pub rooms: HashMap<RoomId, RoomDef>,
    /// Per-room mutable runtime state (trigger flags, revealed exits).
    pub room_states: HashMap<RoomId, RoomState>,
    /// Current room for each connected player (account_id -> room_id).
    pub player_positions: HashMap<String, RoomId>,
}

/// Event broadcast to all players in a room via `tokio::sync::broadcast`.
#[derive(Debug, Clone)]
pub struct WorldEvent {
    pub message: String,
}
