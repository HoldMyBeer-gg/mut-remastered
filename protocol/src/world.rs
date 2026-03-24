use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClientMsg {
    /// Move in a direction. Accepts full names or aliases: "n", "north", "s", "south", "e", "east", "w", "west", "u", "up", "d", "down"
    Move { direction: String },
    /// Re-display current room description
    Look,
    /// Examine an object/feature in the room (returns lore text or "nothing of note")
    Examine { target: String },
    /// Freeform trigger activation command (e.g., "pull lever", "read sign")
    Interact { command: String },
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
}
