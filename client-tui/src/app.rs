//! Application state machine and game state.

use std::collections::HashMap;

use protocol::character::CharacterSummary;

/// Top-level application state.
pub enum AppScreen {
    /// Connecting to server.
    Connecting,
    /// Login screen: entering username/password.
    Login(LoginState),
    /// Character selection screen.
    CharacterSelect(CharSelectState),
    /// Main game interface.
    InGame(GameState),
}

/// Login screen state.
pub struct LoginState {
    pub username: String,
    pub password: String,
    /// Which field is focused: 0 = username, 1 = password.
    pub focus: u8,
    pub error_message: Option<String>,
    pub registering: bool,
}

impl LoginState {
    pub fn new() -> Self {
        Self {
            username: String::new(),
            password: String::new(),
            focus: 0,
            error_message: None,
            registering: false,
        }
    }
}

/// Character selection state.
pub struct CharSelectState {
    pub characters: Vec<CharacterSummary>,
    pub selected_index: usize,
    pub creating: bool,
    pub create_name: String,
    pub create_race: usize,
    pub create_class: usize,
    pub create_gender: usize,
    pub create_focus: u8,
    pub error_message: Option<String>,
}

impl CharSelectState {
    pub fn new() -> Self {
        Self {
            characters: Vec::new(),
            selected_index: 0,
            creating: false,
            create_name: String::new(),
            create_race: 0,
            create_class: 0,
            create_gender: 0,
            create_focus: 0,
            error_message: None,
        }
    }
}

pub const RACES: &[&str] = &["human", "elf", "dwarf", "halfling", "orc", "gnome", "half_elf", "tiefling"];
pub const RACE_NAMES: &[&str] = &["Human", "Elf", "Dwarf", "Halfling", "Orc", "Gnome", "Half-Elf", "Tiefling"];
pub const CLASSES: &[&str] = &["warrior", "ranger", "cleric", "mage", "rogue"];
pub const CLASS_NAMES: &[&str] = &["Warrior", "Ranger", "Cleric", "Mage", "Rogue"];
pub const GENDERS: &[&str] = &["male", "female", "non_binary"];
pub const GENDER_NAMES: &[&str] = &["Male", "Female", "Non-Binary"];

/// Main game state.
pub struct GameState {
    pub character_id: String,
    pub character_name: String,
    /// Current room info.
    pub room_name: String,
    pub room_description: String,
    pub room_exits: Vec<String>,
    pub room_id: String,
    pub players_here: Vec<String>,
    /// Scrollable game log (newest at bottom).
    pub game_log: Vec<String>,
    /// Vitals.
    pub hp: i32,
    pub max_hp: i32,
    pub mana: i32,
    pub max_mana: i32,
    pub stamina: i32,
    pub max_stamina: i32,
    pub xp: i32,
    pub level: i32,
    /// Input line.
    pub input: String,
    /// Command history.
    pub history: Vec<String>,
    pub history_index: Option<usize>,
    /// Explored rooms for minimap: room_id -> (name, exits).
    pub explored_rooms: HashMap<String, ExploredRoom>,
    /// Room connection graph: (from_room_id, direction) -> to_room_id.
    /// Populated when the player moves (from MoveOk responses).
    pub room_connections: HashMap<(String, String), String>,
    /// Last movement direction (set when a move command is sent, before MoveOk response).
    pub last_move_direction: Option<String>,
    /// Monsters visible in current room.
    pub monsters_here: Vec<String>,
    /// Scroll offset for game log (0 = bottom).
    pub log_scroll: u16,
    /// Frame counter for animations.
    pub frame: u64,
    /// Camera facing direction in radians (0 = north, smoothly interpolated).
    pub camera_angle: f64,
    /// Target camera angle (set on room change based on entry direction).
    pub camera_target_angle: f64,
    /// Camera "walk" animation progress (0.0 = just entered, 1.0 = settled).
    pub camera_walk: f64,
    /// Whether camera is currently animating a transition.
    pub camera_animating: bool,
    /// Whether we're currently in combat.
    pub in_combat: bool,
    /// Current combat round number.
    pub combat_round: u32,
    /// Frame when last round resolved (for GCD countdown display).
    pub last_round_frame: u64,
}

pub struct ExploredRoom {
    pub name: String,
    pub exits: Vec<String>,
}

impl GameState {
    pub fn new(character_id: String, character_name: String) -> Self {
        Self {
            character_id,
            character_name,
            room_name: String::new(),
            room_description: String::new(),
            room_exits: Vec::new(),
            room_id: String::new(),
            players_here: Vec::new(),
            game_log: Vec::new(),
            hp: 0,
            max_hp: 0,
            mana: 0,
            max_mana: 0,
            stamina: 0,
            max_stamina: 0,
            xp: 0,
            level: 0,
            input: String::new(),
            history: Vec::new(),
            history_index: None,
            explored_rooms: HashMap::new(),
            room_connections: HashMap::new(),
            last_move_direction: None,
            monsters_here: Vec::new(),
            log_scroll: 0,
            frame: 0,
            camera_angle: 0.0,
            camera_target_angle: 0.0,
            camera_walk: 1.0,
            camera_animating: false,
            in_combat: false,
            combat_round: 0,
            last_round_frame: 0,
        }
    }

    /// Add a message to the game log.
    pub fn log(&mut self, msg: String) {
        self.game_log.push(msg);
        // Keep last 500 entries
        if self.game_log.len() > 500 {
            self.game_log.remove(0);
        }
        self.log_scroll = 0; // Auto-scroll to bottom
    }

    /// Record the current room for minimap.
    pub fn record_room(&mut self) {
        self.explored_rooms.insert(
            self.room_id.clone(),
            ExploredRoom {
                name: self.room_name.clone(),
                exits: self.room_exits.clone(),
            },
        );
    }
}
