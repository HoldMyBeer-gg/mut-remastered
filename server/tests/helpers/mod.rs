use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use protocol::auth::{ClientMsg, ServerMsg};
use protocol::codec::{decode_message, encode_message, NS_AUTH, NS_CHAR, NS_WORLD};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{RwLock, broadcast};

use server::world::types::{RoomId, WorldEvent};

/// A test server started on a random OS-assigned port.
///
/// Multiple constructors are provided for different test scenarios:
/// - `start()` — empty world, in-memory DB (for auth tests)
/// - `start_with_world()` — real zones loaded from TOML, in-memory DB, default spawn
/// - `start_with_db(db_url)` — real zones loaded, file-based DB (for persistence tests)
/// - `start_with_spawn(room_id)` — real zones loaded, in-memory DB, custom spawn room
pub struct TestServer {
    pub addr: String,
    /// Keep handle alive so the server task continues running.
    _handle: tokio::task::JoinHandle<()>,
}

impl TestServer {
    /// Start a real server on a random port with an isolated in-memory database.
    ///
    /// Uses an empty World (no zones loaded). Auth integration tests use this variant
    /// since they don't exercise world commands.
    pub async fn start() -> Self {
        let db_url = format!(
            "sqlite:file:testdb_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().simple()
        );
        Self::start_inner(&db_url, None, None).await
    }

    /// Start a server with real zone TOML files loaded and a default spawn room.
    ///
    /// Uses an in-memory DB (tests run in isolation). Zone files are resolved via
    /// `CARGO_MANIFEST_DIR` (set by Cargo at compile time to the server/ directory).
    pub async fn start_with_world() -> Self {
        let db_url = format!(
            "sqlite:file:testdb_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().simple()
        );
        Self::start_inner(&db_url, Some(zones_dir()), None).await
    }

    /// Start a server with real zone TOML files and a file-based SQLite DB.
    ///
    /// Used for persistence tests where two server instances share the same DB file.
    pub async fn start_with_db(db_url: &str) -> Self {
        Self::start_inner(db_url, Some(zones_dir()), None).await
    }

    /// Start a server with real zone TOML files and a custom default spawn room.
    ///
    /// Used for tests that need players to start in a specific zone (e.g., newbie zone).
    /// The spawn_room overrides the compiled-in DEFAULT_SPAWN_ROOM via `World::default_spawn`.
    pub async fn start_with_spawn(spawn_room: &str) -> Self {
        let db_url = format!(
            "sqlite:file:testdb_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().simple()
        );
        Self::start_inner(&db_url, Some(zones_dir()), Some(spawn_room.to_string())).await
    }

    /// Internal constructor used by all public variants.
    async fn start_inner(
        db_url: &str,
        zones: Option<PathBuf>,
        spawn_override: Option<String>,
    ) -> Self {
        let pool = server::db::init_db(db_url)
            .await
            .expect("failed to init test database");

        // Load world from zone TOML files (or use empty world for auth tests)
        let mut world = if let Some(zones_path) = zones {
            server::world::loader::load_world(&zones_path, &pool)
                .await
                .expect("failed to load world zones")
        } else {
            server::world::types::World::default()
        };

        // Apply custom spawn override if provided
        if let Some(room_id) = spawn_override {
            world.default_spawn = Some(RoomId(room_id));
        }

        // Create per-room broadcast channels
        let mut channels: HashMap<RoomId, broadcast::Sender<WorldEvent>> = HashMap::new();
        for room_id in world.rooms.keys() {
            let (tx, _rx) = broadcast::channel::<WorldEvent>(32);
            channels.insert(room_id.clone(), tx);
        }

        let world = Arc::new(RwLock::new(world));
        let room_channels = Arc::new(RwLock::new(channels));

        let state = server::net::listener::AppState {
            db: pool,
            session_ttl_secs: 3600,
            world,
            room_channels,
            combat_manager: Arc::new(RwLock::new(server::combat::manager::CombatManager::new())),
            monster_templates: Arc::new(std::collections::HashMap::new()),
            active_monsters: Arc::new(RwLock::new(std::collections::HashMap::new())),
            respawn_timers: Arc::new(RwLock::new(Vec::new())),
        };

        // Bind to port 0 to get an OS-assigned free port
        let probe = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind probe listener");
        let addr = probe.local_addr().expect("get probe addr").to_string();
        drop(probe);

        let addr_clone = addr.clone();
        let handle = tokio::spawn(async move {
            server::net::listener::run_listener(&addr_clone, state)
                .await
                .expect("server task failed");
        });

        // Allow the server task a moment to bind and enter the accept loop.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        TestServer {
            addr,
            _handle: handle,
        }
    }

    /// Open a new TCP connection to this test server.
    pub async fn connect(&self) -> TestClient {
        let stream = TcpStream::connect(&self.addr)
            .await
            .expect("failed to connect to test server");
        TestClient { stream }
    }
}

/// Compute the path to the world/zones directory using CARGO_MANIFEST_DIR.
///
/// `CARGO_MANIFEST_DIR` is set by Cargo at compile time to the directory
/// containing the crate's Cargo.toml (i.e., `server/`). From there we navigate
/// up one level and then into `world/zones`.
fn zones_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../world/zones")
}

/// A connected test client that wraps a `TcpStream` and provides typed send/recv helpers.
pub struct TestClient {
    pub stream: TcpStream,
}

impl TestClient {
    /// Construct a `TestClient` from an already-connected `TcpStream`.
    ///
    /// Used by the concurrent connections test where streams are created independently.
    pub fn from_stream(stream: TcpStream) -> Self {
        TestClient { stream }
    }

    /// Register and login in one call. Returns the session token.
    pub async fn register_and_login(&mut self, username: &str, password: &str) -> String {
        // Register
        self.send(&ClientMsg::Register {
            username: username.to_string(),
            password: password.to_string(),
        })
        .await;
        let _reg_resp = self.recv().await; // RegisterOk

        // Login
        self.send(&ClientMsg::Login {
            username: username.to_string(),
            password: password.to_string(),
        })
        .await;
        let login_resp = self.recv().await;
        match login_resp {
            ServerMsg::LoginOk { session_token } => session_token,
            other => panic!("register_and_login: expected LoginOk, got {:?}", other),
        }
    }

    /// Encode and send an auth `ClientMsg` to the server.
    pub async fn send(&mut self, msg: &ClientMsg) {
        let bytes = encode_message(NS_AUTH, msg).expect("failed to encode ClientMsg");
        self.stream
            .write_all(&bytes)
            .await
            .expect("failed to write to test server");
    }

    /// Read the next length-prefixed frame from the server and decode it as an auth `ServerMsg`.
    pub async fn recv(&mut self) -> ServerMsg {
        let mut len_buf = [0u8; 4];
        self.stream
            .read_exact(&mut len_buf)
            .await
            .expect("failed to read length prefix from test server");
        let payload_len = u32::from_le_bytes(len_buf) as usize;

        let mut payload = vec![0u8; payload_len];
        self.stream
            .read_exact(&mut payload)
            .await
            .expect("failed to read payload from test server");

        decode_message::<ServerMsg>(NS_AUTH, &payload).expect("failed to decode ServerMsg")
    }

    /// Encode and send a world `ClientMsg` to the server.
    pub async fn send_world(&mut self, msg: &protocol::world::ClientMsg) {
        let bytes = encode_message(NS_WORLD, msg).expect("failed to encode world ClientMsg");
        self.stream
            .write_all(&bytes)
            .await
            .expect("failed to write world msg to test server");
    }

    /// Read the next length-prefixed frame and decode it as a world `ServerMsg`.
    pub async fn recv_world(&mut self) -> protocol::world::ServerMsg {
        let mut len_buf = [0u8; 4];
        self.stream
            .read_exact(&mut len_buf)
            .await
            .expect("failed to read world msg length prefix");
        let payload_len = u32::from_le_bytes(len_buf) as usize;

        let mut payload = vec![0u8; payload_len];
        self.stream
            .read_exact(&mut payload)
            .await
            .expect("failed to read world msg payload");

        decode_message::<protocol::world::ServerMsg>(NS_WORLD, &payload)
            .expect("failed to decode world ServerMsg")
    }

    /// Send a Move command with the given direction string.
    pub async fn send_move(&mut self, direction: &str) {
        self.send_world(&protocol::world::ClientMsg::Move {
            direction: direction.to_string(),
        })
        .await;
    }

    /// Send a Look command.
    pub async fn send_look(&mut self) {
        self.send_world(&protocol::world::ClientMsg::Look).await;
    }

    /// Send an Examine command with the given target.
    pub async fn send_examine(&mut self, target: &str) {
        self.send_world(&protocol::world::ClientMsg::Examine {
            target: target.to_string(),
        })
        .await;
    }

    /// Send an Interact command with the given command string.
    pub async fn send_interact(&mut self, command: &str) {
        self.send_world(&protocol::world::ClientMsg::Interact {
            command: command.to_string(),
        })
        .await;
    }

    /// Encode and send a character `ClientMsg` to the server.
    pub async fn send_char(&mut self, msg: &protocol::character::ClientMsg) {
        let bytes = encode_message(NS_CHAR, msg).expect("failed to encode character ClientMsg");
        self.stream
            .write_all(&bytes)
            .await
            .expect("failed to write character msg to test server");
    }

    /// Read the next length-prefixed frame and decode it as a character `ServerMsg`.
    pub async fn recv_char(&mut self) -> protocol::character::ServerMsg {
        let mut len_buf = [0u8; 4];
        self.stream
            .read_exact(&mut len_buf)
            .await
            .expect("failed to read character msg length prefix");
        let payload_len = u32::from_le_bytes(len_buf) as usize;

        let mut payload = vec![0u8; payload_len];
        self.stream
            .read_exact(&mut payload)
            .await
            .expect("failed to read character msg payload");

        decode_message::<protocol::character::ServerMsg>(NS_CHAR, &payload)
            .expect("failed to decode character ServerMsg")
    }

    /// Full setup: register, login, create a default warrior character, select it.
    /// Returns (session_token, character_id).
    pub async fn full_setup(
        &mut self,
        username: &str,
        password: &str,
        char_name: &str,
    ) -> (String, String) {
        // Register
        self.send(&ClientMsg::Register {
            username: username.to_string(),
            password: password.to_string(),
        })
        .await;
        let _reg = self.recv().await;

        // Login
        self.send(&ClientMsg::Login {
            username: username.to_string(),
            password: password.to_string(),
        })
        .await;
        let login_resp = self.recv().await;
        let session_token = match login_resp {
            ServerMsg::LoginOk { session_token } => session_token,
            other => panic!("full_setup: expected LoginOk, got {:?}", other),
        };

        // Create character (standard warrior build: 15 STR, 14 CON, 13 DEX, 12 WIS, 10 INT, 8 CHA = 9+7+5+4+2+0 = 27)
        self.send_char(&protocol::character::ClientMsg::CharacterCreate {
            name: char_name.to_string(),
            race: "human".to_string(),
            class: "warrior".to_string(),
            gender: "male".to_string(),
            ability_scores: [15, 13, 14, 10, 12, 8],
            racial_bonus_choices: vec![0, 2], // +1 STR, +1 CON
        })
        .await;
        let create_resp = self.recv_char().await;
        let character_id = match create_resp {
            protocol::character::ServerMsg::CharacterCreateOk { character_id, .. } => character_id,
            other => panic!("full_setup: expected CharacterCreateOk, got {:?}", other),
        };

        // Select character
        self.send_char(&protocol::character::ClientMsg::CharacterSelect {
            character_id: character_id.clone(),
        })
        .await;
        let _selected = self.recv_char().await; // CharacterSelected
        let _room_desc = self.recv_world().await; // Initial RoomDescription
        let _vitals = self.recv_combat().await; // Initial Vitals

        (session_token, character_id)
    }

    /// Encode and send a combat `ClientMsg` to the server.
    pub async fn send_combat(&mut self, msg: &protocol::combat::ClientMsg) {
        let bytes =
            encode_message(protocol::codec::NS_COMBAT, msg).expect("failed to encode combat ClientMsg");
        self.stream
            .write_all(&bytes)
            .await
            .expect("failed to write combat msg to test server");
    }

    /// Read the next length-prefixed frame and decode it as a combat `ServerMsg`.
    pub async fn recv_combat(&mut self) -> protocol::combat::ServerMsg {
        let mut len_buf = [0u8; 4];
        self.stream
            .read_exact(&mut len_buf)
            .await
            .expect("failed to read combat msg length prefix");
        let payload_len = u32::from_le_bytes(len_buf) as usize;

        let mut payload = vec![0u8; payload_len];
        self.stream
            .read_exact(&mut payload)
            .await
            .expect("failed to read combat msg payload");

        decode_message::<protocol::combat::ServerMsg>(protocol::codec::NS_COMBAT, &payload)
            .expect("failed to decode combat ServerMsg")
    }
}
