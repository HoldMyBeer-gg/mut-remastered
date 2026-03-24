# Phase 02: World and Movement - Research

**Researched:** 2026-03-24
**Domain:** Rust world state management, TOML data loading, SQLite persistence, room-based broadcast
**Confidence:** HIGH

---

## Summary

Phase 2 builds the persistent world on top of the Phase 1 server foundation. The primary technical work is: (1) a TOML-based room data loader that reads zone files from disk and populates an in-memory `World` struct wrapped in `Arc<RwLock<World>>`; (2) a command dispatcher extension to `ConnectionActor` that handles Move, Look, Examine, and Interact messages; (3) SQLite persistence for player positions and mutable world state using new migrations; and (4) a per-room `tokio::sync::broadcast` channel for fan-out of world events (lever pulled, player moved) to all connected players in the same room.

The data-driven trigger system (D-10, D-11) maps naturally to TOML with an array of trigger tables per room — each trigger defines an activation command, an optional condition, and an effects list. The generic handler in the server evaluates these without any per-room hardcoded logic. The newbie area (D-07, D-08, D-09) is a zone TOML file with a `hints` field on rooms; the server filters hints by a per-account `tutorial_complete` flag.

The key architectural decision already locked in CONTEXT.md is that the in-memory `World` is the hot-path query target; SQLite is only written to on mutations (position change, trigger fired) and read on startup to overlay persisted state onto TOML definitions. This avoids per-movement database round-trips.

**Primary recommendation:** Use `toml 1.1.0` (current stable) with serde for all room data loading. Extend `AppState` with `Arc<RwLock<World>>` and a `HashMap<RoomId, broadcast::Sender<WorldEvent>>` for fan-out. Add new protocol message types to the `protocol` crate before implementing any server handlers.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Rooms defined as TOML data files loaded at startup (one file per zone or region). Room definitions include descriptions, exits, lore text, and trigger definitions. World authoring in version-controllable text files separate from code.
- **D-02:** Runtime state changes persisted in SQLite. On startup: load TOML room definitions, then overlay any persisted state mutations from the database.
- **D-03:** Player positions stored in SQLite (account_id → room_id mapping) to satisfy WRLD-03 (survive server restart).
- **D-04:** Phase 2 uses event-driven command processing — player types a command, server processes it immediately. No fixed-tick loop yet.
- **D-05:** World state (rooms, player positions, object states) lives in an in-memory data structure (Arc<RwLock<World>> or similar) as single source of truth during runtime. SQLite is the persistence layer, not the query layer for hot-path operations.
- **D-06:** Fixed-tick ECS introduced in Phase 3 when combat round timers require it. Phase 2 does not need time-based systems.
- **D-07:** Newbie area is freely explorable (not forced linear corridor). Contextual hints when players enter rooms or attempt commands.
- **D-08:** Newbie area has 5-8 rooms that naturally teach movement, looking, examining objects, and interacting with triggers before leading to the wider world.
- **D-09:** Hints stored in room data (a `hints` field in the TOML). Server sends hints only to players who haven't completed the tutorial flag.
- **D-10:** Room TOML files define triggers as data: activation command, optional condition, and effects (set state key/value, reveal exit, broadcast message to room).
- **D-11:** A generic trigger handler processes data-driven triggers. No per-room hardcoded logic.
- **D-12:** Trigger state changes broadcast to all players in the room (WRLD-06).

### Claude's Discretion
- Room TOML file structure and field names
- How zones/regions are organized (single file per zone, directory structure)
- Internal data structures for the world state
- Protocol message types for movement, room descriptions, and world events
- Migration schema for world state persistence tables

### Deferred Ideas (OUT OF SCOPE)
- None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| WRLD-01 | Rooms have rich text descriptions and visible exits | TOML `description` + `exits` fields loaded into Room struct; `Look` command returns RoomDescription message |
| WRLD-02 | User can move between rooms using cardinal directions (n/s/e/w/u/d) and aliases | `Move { direction }` ClientMsg; direction enum with aliases (n=North, s=South, etc.); exit lookup in Room.exits HashMap |
| WRLD-03 | World state persists across server restarts and player disconnects | `player_positions` SQLite table; written on move + logout; read on login to restore position |
| WRLD-04 | Newbie area guides first-time players through commands in a safe zone | Zone TOML file with `hints` field on rooms; `tutorial_complete` flag in accounts table; hint suppression after flag set |
| WRLD-05 | Rooms contain embedded lore that rewards exploration | `lore` field in TOML loaded into Room struct; `Examine` command returns lore text |
| WRLD-06 | Player actions have persistent consequences that affect world state for all players | Data-driven trigger system in TOML; generic handler evaluates triggers; broadcast::Sender per room fans out WorldEvent |
</phase_requirements>

---

## Project Constraints (from CLAUDE.md)

Directives the planner MUST verify compliance with:

- Language: Rust (stable, 1.87+). Installed: rustc 1.92.0.
- Async runtime: Tokio 1.x (workspace dependency, verified 1.50.0)
- Binary protocol: postcard (replaces bincode — RUSTSEC-2025-0141)
- Database: SQLx 0.8.x with SQLite (existing pattern in server/src/db.rs)
- Serialization: serde 1.0 (workspace dependency)
- No terminal-specific protocols
- Actor-per-session pattern (no shared mutable state except Arc-wrapped shared state)
- Protocol crate: all ClientMsg/ServerMsg variants defined in `protocol/` crate, never in server or client crates directly
- WAL mode + foreign keys enabled on SQLite pool (existing db.rs pattern)
- Do NOT use: tui-rs, termion, Rocket, Diesel, bcrypt/PBKDF2, ws crate

---

## Standard Stack

### Core (Phase 2 Additions)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| toml | 1.1.0 | Parse zone TOML files into Room structs | Official Rust TOML library; serde-compatible; supports all TOML features including arrays of tables (trigger definitions); published 2026-03-23 |
| tokio | 1.x (workspace) | broadcast::channel for per-room fan-out | Already in workspace; broadcast::Sender stored per-room; each ConnectionActor holds a Receiver |

### Already in Workspace (Reused)
| Library | Version | Purpose |
|---------|---------|---------|
| sqlx | 0.8.6 | New migrations: player_positions, world_state tables |
| serde | 1.0 (workspace) | Deserialize TOML room data into structs |
| postcard | 1.x (workspace) | Protocol encode/decode for new message types |
| tokio | 1.x (workspace) | RwLock for world state, broadcast for room events |
| tracing | 0.1 (workspace) | Span-based logging for movement and trigger events |
| anyhow | 1.x (workspace) | Error propagation in world loader and handlers |
| uuid | 1.x | Already in server; used for room IDs if needed |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| toml 1.1.0 | serde_json (JSON zone files) | JSON is more familiar but TOML is more author-friendly for multiline room descriptions; TOML is the idiomatic Rust config format |
| toml 1.1.0 | ron (Rusty Object Notation) | RON is more expressive but requires custom tooling; TOML has broad editor support for world authors |
| Arc<RwLock<World>> | DashMap | DashMap offers sharded concurrent HashMaps but adds a dependency for negligible benefit at MUD player counts; RwLock is sufficient |
| broadcast::channel per room | mpsc per player + fan-out task | More complex topology; broadcast is the correct primitive for 1-to-N room events |

**Installation (server Cargo.toml additions):**
```toml
toml = "1.1"
```

**Version verification:** toml 1.1.0 confirmed against crates.io API on 2026-03-24.

---

## Architecture Patterns

### Recommended Directory Structure (Phase 2 additions)
```
mut_remastered/
├── world/                        # World data (version-controlled)
│   └── zones/
│       ├── newbie/
│       │   └── zone.toml         # 5-8 room newbie area
│       └── starting_village/
│           └── zone.toml         # First wider-world zone
├── server/
│   ├── migrations/
│   │   ├── 001_accounts.sql      # Existing
│   │   ├── 002_player_positions.sql  # NEW: account_id → room_id
│   │   └── 003_world_state.sql   # NEW: per-room trigger state kv store
│   └── src/
│       ├── world/
│       │   ├── mod.rs            # pub use
│       │   ├── loader.rs         # Load zone TOML files → World struct
│       │   ├── types.rs          # Room, Exit, Trigger, Direction, WorldEvent
│       │   └── commands.rs       # handle_move, handle_look, handle_examine, handle_interact
│       ├── net/
│       │   └── listener.rs       # AppState gains world: Arc<RwLock<World>>
│       │                         #             + room_channels: Arc<RwLock<RoomChannels>>
│       └── session/
│           └── actor.rs          # handle_message gains Move/Look/Examine/Interact arms
├── protocol/
│   └── src/
│       ├── auth.rs               # Existing
│       └── world.rs              # NEW: Move, Look, Examine, Interact, RoomDescription, WorldEvent
```

### Pattern 1: TOML Zone File Format
**What:** Each zone is a TOML file with an array of room tables. Exits are a TOML inline table mapping direction strings to room IDs. Triggers are arrays of inline tables with command, optional condition, and effects.

**Example zone.toml:**
```toml
# world/zones/newbie/zone.toml
zone_id = "newbie"
zone_name = "The Newcomer's Crossing"

[[rooms]]
id = "newbie:entrance"
name = "The Crossing Gate"
description = """
You stand at a weathered stone gate draped in ivy. Carved runes above the arch
read: "All who seek wisdom must first learn to walk." A cobbled path leads north
into a quiet courtyard, and a notice board hangs to your east."""
lore = """
The runes are in Old Elvish, a tongue seldom spoken since the Age of Ruin.
They suggest this place was once an academy for young adventurers."""
hints = [
    "Try typing 'north' or just 'n' to walk through the gate.",
    "Type 'look' at any time to see your surroundings again.",
]

[rooms.exits]
north = "newbie:courtyard"
east  = "newbie:notice_board"

[[rooms.triggers]]
command = "read runes"
effects = [
    { kind = "message", text = "You study the runes. They read: 'All who seek wisdom must first learn to walk.'" },
]

[[rooms]]
id = "newbie:courtyard"
name = "The Quiet Courtyard"
description = """
A mossy fountain burbles in the center of a sun-dappled courtyard. Paths lead
south back to the gate, east toward a practice hall, and north through an archway
into the wider world."""
lore = "The fountain's basin is carved with scenes from an ancient dungeon expedition."

[rooms.exits]
south = "newbie:entrance"
east  = "newbie:practice_hall"
north = "starting_village:market_square"

[[rooms.triggers]]
command = "examine fountain"
condition = { key = "fountain_touched", value = "false" }
effects = [
    { kind = "set_state", key = "fountain_touched", value = "true" },
    { kind = "message", text = "You touch the cool water. The carvings glow briefly." },
    { kind = "broadcast", text = "{player} touches the ancient fountain. It glows briefly." },
]
```

**When to use:** All room definitions. The `[[rooms]]` array-of-tables pattern maps directly to `Vec<RoomDef>` in Rust.

### Pattern 2: World Data Structures
**What:** Two-layer design — `RoomDef` (static TOML data, never mutated at runtime) and `RoomState` (runtime state, mutable, persisted to SQLite). The `World` struct owns both layers plus a lookup map from room ID to room index.

**Example:**
```rust
// server/src/world/types.rs

use std::collections::HashMap;

/// A newtype wrapper for room IDs prevents mixing room IDs with other strings.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RoomId(pub String);

#[derive(Debug, Clone)]
pub enum Direction {
    North, South, East, West, Up, Down,
}

impl Direction {
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
}

/// Static room definition loaded from TOML. Never mutated after startup.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RoomDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub lore: Option<String>,
    pub hints: Option<Vec<String>>,
    pub exits: HashMap<String, String>,  // direction key → room_id
    pub triggers: Option<Vec<TriggerDef>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct TriggerDef {
    pub command: String,
    pub condition: Option<TriggerCondition>,
    pub effects: Vec<TriggerEffect>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct TriggerCondition {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TriggerEffect {
    Message { text: String },
    Broadcast { text: String },
    SetState { key: String, value: String },
    RevealExit { direction: String, target: String },
}

/// Mutable state for a room at runtime. Overlaid from SQLite on startup.
#[derive(Debug, Default, Clone)]
pub struct RoomState {
    /// Key-value store for trigger state (e.g., "lever_state" = "pulled").
    pub kv: HashMap<String, String>,
    /// Dynamic exits added by triggers (merged with RoomDef.exits for lookups).
    pub extra_exits: HashMap<String, RoomId>,
}

/// The live world: room definitions + mutable state + player positions.
pub struct World {
    /// All static room definitions, keyed by RoomId.
    pub rooms: HashMap<RoomId, RoomDef>,
    /// Per-room mutable state (trigger outcomes, dynamic exits).
    pub room_states: HashMap<RoomId, RoomState>,
    /// Current room for each logged-in account.
    pub player_positions: HashMap<String, RoomId>,  // account_id → room_id
}
```

### Pattern 3: World Loader
**What:** At server startup, scan the `world/zones/` directory, parse each `zone.toml` file with the `toml` crate, then overlay persisted state from SQLite. Result is inserted into `Arc<RwLock<World>>`.

**Example:**
```rust
// server/src/world/loader.rs
use std::fs;
use std::path::Path;
use std::collections::HashMap;

use serde::Deserialize;
use crate::world::types::{RoomDef, RoomId, RoomState, World};

#[derive(Deserialize)]
struct ZoneFile {
    zone_id: String,
    zone_name: String,
    rooms: Vec<RoomDef>,
}

pub async fn load_world(
    zones_dir: &Path,
    pool: &sqlx::SqlitePool,
) -> anyhow::Result<World> {
    let mut rooms = HashMap::new();

    // Load all zone.toml files
    for entry in fs::read_dir(zones_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            let content = fs::read_to_string(&path)?;
            let zone: ZoneFile = toml::from_str(&content)
                .map_err(|e| anyhow::anyhow!("failed to parse {:?}: {e}", path))?;
            for room in zone.rooms {
                rooms.insert(RoomId(room.id.clone()), room);
            }
        }
    }

    // Overlay persisted state from SQLite
    let state_rows = sqlx::query!("SELECT room_id, state_key, state_value FROM world_state")
        .fetch_all(pool)
        .await?;

    let mut room_states: HashMap<RoomId, RoomState> = HashMap::new();
    for row in state_rows {
        let state = room_states.entry(RoomId(row.room_id)).or_default();
        state.kv.insert(row.state_key, row.state_value);
    }

    // Load player positions
    let position_rows = sqlx::query!("SELECT account_id, room_id FROM player_positions")
        .fetch_all(pool)
        .await?;
    let player_positions: HashMap<String, RoomId> = position_rows
        .into_iter()
        .map(|r| (r.account_id, RoomId(r.room_id)))
        .collect();

    Ok(World { rooms, room_states, player_positions })
}
```

### Pattern 4: AppState Extension for World
**What:** `AppState` gains two new fields: `world: Arc<RwLock<World>>` as the single source of truth for runtime world data, and `room_channels: Arc<RwLock<HashMap<RoomId, broadcast::Sender<WorldEvent>>>>` for per-room fan-out.

**Example:**
```rust
// server/src/net/listener.rs (extended)
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{RwLock, broadcast};
use crate::world::types::{World, RoomId, WorldEvent};

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::SqlitePool,
    pub session_ttl_secs: i64,
    pub world: Arc<RwLock<World>>,
    pub room_channels: Arc<RwLock<HashMap<RoomId, broadcast::Sender<WorldEvent>>>>,
}
```

**Room channel management:** When a player enters a room, call `sender.subscribe()` to get a `Receiver`. Store the `Receiver` in the `ConnectionActor`. In the actor's main loop, `tokio::select!` on both the TCP reader (incoming commands) and the room `Receiver` (incoming world events).

### Pattern 5: ConnectionActor Extension with select!
**What:** The existing `run()` loop reads TCP frames. Phase 2 adds a `tokio::select!` in the main loop so the actor can simultaneously wait for a command from the client AND receive world events broadcast to the current room.

**Example:**
```rust
// server/src/session/actor.rs (Phase 2 extension sketch)
use tokio::sync::broadcast;
use crate::world::types::WorldEvent;

pub struct ConnectionActor {
    // ... existing fields ...
    room_receiver: Option<broadcast::Receiver<WorldEvent>>,
}

pub async fn run(&mut self) -> anyhow::Result<()> {
    loop {
        tokio::select! {
            frame = self.read_frame() => {
                match frame? {
                    Some(bytes) => {
                        let msg = decode_message::<ClientMsg>(&bytes)?;
                        self.handle_message(msg).await?;
                    }
                    None => break,  // EOF
                }
            }
            event = async {
                if let Some(rx) = &mut self.room_receiver {
                    rx.recv().await
                } else {
                    std::future::pending().await
                }
            } => {
                if let Ok(event) = event {
                    self.send_world_event(event).await?;
                }
                // RecvError::Lagged: skip, continue
            }
        }
    }
    self.cleanup().await;
    Ok(())
}
```

### Pattern 6: Protocol Extensions
**What:** New message types added to `protocol/src/world.rs`. The existing `ClientMsg` and `ServerMsg` enums in `protocol/src/auth.rs` are extended OR a separate `world.rs` module is created and the top-level `ClientMsg`/`ServerMsg` enums become wrappers. The cleanest approach for Phase 2 is a unified `ClientMsg`/`ServerMsg` in the protocol crate that adds new variants.

**New ClientMsg variants:**
```rust
// protocol/src/world.rs — add to ClientMsg enum
Move { direction: String },   // "n", "north", "s", etc.
Look,                          // re-display current room
Examine { target: String },    // examine <object/feature>
Interact { command: String },  // freeform trigger activation ("pull lever", "read sign")
```

**New ServerMsg variants:**
```rust
// protocol/src/world.rs — add to ServerMsg enum
RoomDescription {
    room_id: String,
    name: String,
    description: String,
    exits: Vec<String>,     // ["north", "east"]
    hints: Vec<String>,     // empty if tutorial complete
    players_here: Vec<String>,  // account names of other players in room
},
MoveOk {
    from_room: String,
    to_room: String,
},
MoveFail { reason: String },     // "There is no exit to the north."
ExamineResult { text: String },  // lore text or "You find nothing of note."
WorldEvent {
    message: String,    // rendered text to display inline
},
```

### Anti-Patterns to Avoid
- **Per-move SQLite reads:** Never query the DB to get a player's current room or room exits during normal movement. The in-memory `World` is the hot path (D-05). Only write to SQLite on state mutations.
- **Blocking on `RwLock::write()` in the hot path:** If the write lock is held for DB persistence, it blocks all readers. Use a write-then-async pattern: grab the write lock, mutate the in-memory state, release the lock, then write to SQLite asynchronously.
- **Hardcoded trigger logic in match arms:** Violates D-11. All trigger behavior must come from the TOML data. The generic handler evaluates `TriggerDef` structs; it never has `if room_id == "newbie:courtyard"` logic.
- **Sending hints to every player on every `Look`:** Only send hints if `tutorial_complete = false` for the account. Check the account flag from session state, not from a DB query on each `Look`.
- **Ignoring `RecvError::Lagged` on broadcast receivers:** A lagged player has missed world events. Log a warning and continue — do not disconnect the player. Missing a few events is acceptable in Phase 2; the player can re-`look` to resync.
- **Room ID strings without newtype:** Using raw `String` for room IDs throughout makes it easy to accidentally pass a zone name or account ID. Use the `RoomId(String)` newtype from day one.
- **Zone files discovered at request time:** Load all zones at startup. Never read TOML files in response to a player command — this adds disk I/O to the hot path and breaks the startup-time validation guarantee.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| TOML parsing | Custom text parser | `toml 1.1.0` with serde | TOML spec has edge cases (multiline strings, datetime, inline tables); hand-rolled parsers miss them |
| Direction alias mapping | Custom string-matching switch | `Direction::from_str` in the types module | Centralize once; all commands share it |
| Per-room pub/sub | Custom notification list with mutex | `tokio::sync::broadcast::channel` | Handles lagged receivers, clone-able senders, zero-copy for same-type messages |
| Zone file discovery | Custom `readdir` + filtering | `std::fs::read_dir` with extension filter | Simple enough with stdlib; no need for walkdir in Phase 2 |
| Trigger condition evaluation | Complex expression parser | Simple `key == value` string comparison (TriggerCondition struct) | Phase 2 triggers are binary state — a full expression engine (like Lua) is Phase 3+ scope |

**Key insight:** The world data loader is run once at startup; it is allowed to be synchronous (`std::fs::read_to_string`) because it runs before the async listener loop starts. Do not complicate it with async file I/O.

---

## Common Pitfalls

### Pitfall 1: RwLock Write Lock Held During SQLite Async Await
**What goes wrong:** Server deadlocks or panics with "tokio RwLock held across await point."
**Why it happens:** `tokio::sync::RwLock` is async-aware and can be held across awaits, but std::sync::RwLock cannot. If `std::sync::RwLock` is used (accidentally) and held while awaiting a SQLx query, the async runtime blocks the thread.
**How to avoid:** Use `tokio::sync::RwLock` (not `std::sync::RwLock`) for `Arc<RwLock<World>>`. Pattern: acquire write lock → mutate in-memory state → drop guard → then await the SQLx write.
**Warning signs:** Deadlock under concurrent test load; clippy warning "MutexGuard held across await."

### Pitfall 2: Broadcast Receiver Dropped Before First Subscription
**What goes wrong:** Player enters a room, a world event fires immediately, and the player misses it because the `Receiver` was not yet in the actor's `select!` loop.
**Why it happens:** `broadcast::channel` drops old messages when capacity is exceeded; a `Receiver` created after a send misses that send.
**How to avoid:** Subscribe to the room channel (call `sender.subscribe()`) before sending the join notification to the room. The order matters: subscribe → broadcast "PlayerName entered." — not the reverse.
**Warning signs:** Players occasionally miss "X entered the room" messages.

### Pitfall 3: TOML Multiline String Escaping
**What goes wrong:** Room descriptions with backticks, backslashes, or TOML-special characters fail to parse.
**Why it happens:** TOML literal strings (`'...'`) do not process escapes but cannot contain `'`. TOML basic strings (`"..."`) process escapes. TOML multi-line literal strings (`'''...'''`) are safest for prose.
**How to avoid:** Use triple-quoted TOML multi-line literal strings (`'''`) for all `description` and `lore` fields. These accept any character except `'''` and require no escaping.
**Warning signs:** `toml::from_str` returns parse errors on zone files with unusual punctuation.

### Pitfall 4: Zone File Room ID Collisions Across Zones
**What goes wrong:** Two zone files define rooms with the same ID string (e.g., both have `id = "entrance"`). One silently overwrites the other in the `HashMap<RoomId, RoomDef>`.
**Why it happens:** The loader inserts rooms into a flat HashMap keyed by room ID; it doesn't detect duplicates unless explicitly checked.
**How to avoid:** Enforce namespace convention: room IDs must be prefixed with zone ID (`zone_id:room_name`, e.g., `"newbie:entrance"`). Add a validation pass in the loader that returns an error if a room ID collision is detected.
**Warning signs:** Zone loading succeeds but some rooms are unreachable or have wrong descriptions.

### Pitfall 5: Tutorial Hint Flag Not Persisted
**What goes wrong:** Player completes the newbie area, restarts their client, and receives hints again. Or worse, the flag is checked from in-memory state that isn't initialized from the DB on reconnect.
**Why it happens:** The `tutorial_complete` flag lives in the `accounts` table (or a separate `account_flags` table). If the session actor only reads it at login and stores it in actor state, it must be loaded correctly from the DB on every login.
**How to avoid:** Read the `tutorial_complete` flag during the Login handler, alongside existing account lookup. Store it in `ConnectionActor` as a bool field. Persist it to SQLite when the newbie-area completion trigger fires.
**Warning signs:** Integration test shows hints appearing after tutorial flag is set in DB.

### Pitfall 6: AppState Clone Includes World but not Channels
**What goes wrong:** `AppState::Clone` derives correctly but `room_channels` is missing (not added to the struct), so trigger fan-out has no way to broadcast events.
**Why it happens:** Incremental development: `world` is added first, `room_channels` added later, derive-based Clone picks up whatever is in the struct at that point.
**How to avoid:** Add both `world` and `room_channels` fields to `AppState` in the same commit. Test fan-out with an integration test before marking WRLD-06 complete.
**Warning signs:** Trigger fires, state changes in SQLite, but other players in the room don't see the event.

---

## Code Examples

### Zone TOML Deserialization (verified pattern)
```rust
// Source: https://docs.rs/toml/latest/toml/
use serde::Deserialize;

#[derive(Deserialize)]
struct ZoneFile {
    zone_id: String,
    zone_name: String,
    rooms: Vec<RoomDef>,
}

let content = std::fs::read_to_string("world/zones/newbie/zone.toml")?;
let zone: ZoneFile = toml::from_str(&content)?;
```

### Per-Room Broadcast Channel Subscription
```rust
// Source: https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html
use std::collections::HashMap;
use tokio::sync::broadcast;

// At world load time, create one channel per room:
let mut room_channels: HashMap<RoomId, broadcast::Sender<WorldEvent>> = HashMap::new();
for room_id in world.rooms.keys() {
    let (tx, _) = broadcast::channel(32);
    room_channels.insert(room_id.clone(), tx);
}

// When player enters a room, the actor subscribes:
if let Some(sender) = room_channels.get(&current_room_id) {
    let receiver = sender.subscribe();
    self.room_receiver = Some(receiver);
}

// When a trigger fires, broadcast to the room:
if let Some(sender) = room_channels.get(&room_id) {
    let _ = sender.send(WorldEvent { message: rendered_text });
    // send() returns Err only if there are no receivers — safe to ignore
}
```

### Generic Trigger Handler
```rust
// Evaluates triggers for a room against a player's command input.
pub async fn evaluate_triggers(
    command: &str,
    room_id: &RoomId,
    account_id: &str,
    world: &mut World,
    pool: &sqlx::SqlitePool,
) -> Vec<TriggerEffect> {
    let room = match world.rooms.get(room_id) {
        Some(r) => r.clone(),
        None => return vec![],
    };
    let state = world.room_states.entry(room_id.clone()).or_default();

    let mut fired_effects = vec![];
    for trigger in room.triggers.as_deref().unwrap_or(&[]) {
        if trigger.command.to_lowercase() != command.to_lowercase() {
            continue;
        }
        // Check optional condition
        if let Some(cond) = &trigger.condition {
            let current = state.kv.get(&cond.key).map(String::as_str).unwrap_or("false");
            if current != cond.value {
                continue;
            }
        }
        // Apply effects to in-memory state
        for effect in &trigger.effects {
            if let TriggerEffect::SetState { key, value } = effect {
                state.kv.insert(key.clone(), value.clone());
                // Persist to SQLite
                sqlx::query!(
                    "INSERT OR REPLACE INTO world_state (room_id, state_key, state_value)
                     VALUES (?, ?, ?)",
                    room_id.0, key, value
                ).execute(pool).await.ok();
            }
        }
        fired_effects.extend(trigger.effects.clone());
        break; // First matching trigger wins
    }
    fired_effects
}
```

### Migration: Player Positions and World State
```sql
-- server/migrations/002_player_positions.sql
CREATE TABLE IF NOT EXISTS player_positions (
    account_id  TEXT PRIMARY KEY NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    room_id     TEXT NOT NULL,
    updated_at  INTEGER NOT NULL DEFAULT (unixepoch())
);

-- server/migrations/003_world_state.sql
CREATE TABLE IF NOT EXISTS world_state (
    room_id     TEXT NOT NULL,
    state_key   TEXT NOT NULL,
    state_value TEXT NOT NULL,
    updated_at  INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (room_id, state_key)
);
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `toml 0.7/0.8` | `toml 1.1.0` | March 2026 (1.0 released late 2025) | Breaking API changes; use `toml::from_str` not `toml::de::from_str`; TOML 1.1 spec support |
| bincode for protocol | postcard 1.x | RUSTSEC-2025-0141 (Phase 1 decision) | Already handled in Phase 1 |
| `tui-rs` | `ratatui 0.30` | 2023 (archived) | Phase 4 concern, not Phase 2 |

**Deprecated/outdated in this domain:**
- `config` crate (0.14.x): General-purpose config loader. TOML-specific loading with `toml` crate is simpler and gives full control over the struct shape. Avoid for room data.
- `ron` crate: Rusty Object Notation was popular for game data but has poor tooling support and no editor plugins for world authors. TOML is better.

---

## Open Questions

1. **Zone subdirectory vs. flat directory**
   - What we know: D-01 says "one file per zone or region"; Claude's discretion allows defining the structure
   - What's unclear: Whether zones should be in `world/zones/zone_id.toml` (flat) or `world/zones/zone_id/zone.toml` (subdirectory for future per-zone assets like NPC definitions)
   - Recommendation: Use subdirectory (`world/zones/newbie/zone.toml`) to leave room for per-zone files in Phase 3+ without restructuring

2. **Tutorial completion trigger location**
   - What we know: D-09 says hints are only shown when `tutorial_complete = false`; D-10/D-11 say triggers are data-driven
   - What's unclear: Whether the tutorial completion trigger (that sets the flag) is a room trigger (fires when player enters a specific exit) or a server-side check (fires when player leaves the newbie zone)
   - Recommendation: Make it a trigger on the `north` exit of the last newbie room (a special effect kind: `{ kind = "set_tutorial_complete" }`) — keeps the behavior in data, handled generically

3. **broadcast channel capacity per room**
   - What we know: `broadcast::channel(capacity)` requires a fixed capacity; messages beyond capacity are dropped for lagged receivers
   - What's unclear: What capacity is appropriate — too small causes lagged errors on burst events; too large wastes memory
   - Recommendation: Start with 32. A MUD room rarely has more than 10-20 active events per second; 32 gives comfortable headroom. Revisit if load testing shows lagged errors in Phase 3

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust (stable) | All compilation | Yes | 1.92.0 | — |
| SQLite (bundled) | world_state migrations | Yes | bundled via sqlx | — |
| World data files | Loader (world/zones/) | No | — | Create world/zones/ dir + zone files in Wave 0 |
| toml crate | Zone file parsing | No (not yet in Cargo.toml) | 1.1.0 available | — |

**Missing dependencies with no fallback:**
- `world/zones/` directory and zone TOML files — must be created as part of Wave 0 (these are game content, not code)

**Missing dependencies with fallback:**
- `toml` crate — add to `server/Cargo.toml` in Wave 0 setup task before world loader code is written

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in (`cargo test`) + `#[tokio::test]` for async |
| Config file | None — `#[cfg(test)]` modules per file |
| Quick run command | `cargo test -p server` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| WRLD-01 | Look command returns room name, description, exits | integration | `cargo test -p server test_look_returns_room_description` | No — Wave 0 |
| WRLD-02 | Move north to adjacent room, receive new RoomDescription | integration | `cargo test -p server test_move_cardinal_direction` | No — Wave 0 |
| WRLD-02 | Move in direction with no exit returns MoveFail | integration | `cargo test -p server test_move_no_exit` | No — Wave 0 |
| WRLD-02 | Direction aliases (n = north, s = south, etc.) work | unit | `cargo test -p server test_direction_aliases` | No — Wave 0 |
| WRLD-03 | Player position persists after server restart (simulated) | integration | `cargo test -p server test_position_survives_restart` | No — Wave 0 |
| WRLD-04 | Hints appear in newbie area for new player | integration | `cargo test -p server test_hints_shown_to_new_player` | No — Wave 0 |
| WRLD-04 | Hints suppressed after tutorial_complete flag set | integration | `cargo test -p server test_hints_suppressed_after_tutorial` | No — Wave 0 |
| WRLD-05 | Examine returns lore text from room | integration | `cargo test -p server test_examine_returns_lore` | No — Wave 0 |
| WRLD-05 | Examine unknown target returns "nothing of note" | integration | `cargo test -p server test_examine_unknown_target` | No — Wave 0 |
| WRLD-06 | Trigger fires, state persists across reconnect | integration | `cargo test -p server test_trigger_state_persists` | No — Wave 0 |
| WRLD-06 | Trigger broadcasts event to all players in room | integration | `cargo test -p server test_trigger_broadcasts_to_room` | No — Wave 0 |

### Integration Test Pattern (extends existing TestServer)
The existing `TestServer` and `TestClient` in `server/tests/helpers/mod.rs` need:
1. `AppState` updated to include `world` and `room_channels` fields — `TestServer::start()` must load a test world
2. A minimal test zone (in-memory TOML string, not file) for deterministic test scenarios
3. `TestClient` extended with world-command helpers: `send_move()`, `send_look()`, `send_examine()`, `send_interact()`

### Sampling Rate
- **Per task commit:** `cargo test -p server`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `server/tests/world_integration.rs` — covers WRLD-01 through WRLD-06
- [ ] `server/tests/helpers/mod.rs` updated — TestServer gains world + room_channels
- [ ] `world/zones/newbie/zone.toml` — 5-8 room newbie zone (game content)
- [ ] `world/zones/starting_village/zone.toml` — first wider-world zone (minimal, 2-3 rooms)
- [ ] `server/migrations/002_player_positions.sql` — player position persistence
- [ ] `server/migrations/003_world_state.sql` — trigger state kv store
- [ ] `toml = "1.1"` added to `server/Cargo.toml`

---

## Sources

### Primary (HIGH confidence)
- https://crates.io/crates/toml — toml 1.1.0 confirmed 2026-03-24 (crates.io API)
- https://docs.rs/toml/latest/toml/ — `toml::from_str` API, serde struct deserialization pattern
- https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html — broadcast::channel API, Lagged error behavior, Sender::subscribe pattern
- https://docs.rs/tokio/latest/tokio/sync/struct.RwLock.html — tokio::sync::RwLock (async-aware, safe across await points)
- Phase 1 RESEARCH.md — all existing crate versions verified 2026-03-23

### Secondary (MEDIUM confidence)
- https://tokio.rs/tokio/tutorial/channels — Tokio channels tutorial including broadcast pattern
- https://github.com/andreivasiliu/demimud — DemiMUD Rust implementation, room structure patterns
- https://github.com/elderhorst/rust-mud-adventure — Rust MUD reference for exit HashMap pattern
- Tokio actor pattern (https://ryhl.io/blog/actors-with-tokio/) — select! loop extension for broadcast receivers

### Tertiary (LOW confidence — verify before use)
- MUD newbie area design patterns (circlemud.org builder's manual) — onboarding philosophy; Phase 2's specific approach is already locked by D-07 through D-09

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — toml 1.1.0 verified against crates.io API on 2026-03-24; all other crates already in workspace from Phase 1 with verified versions
- Architecture: HIGH — Arc<RwLock<World>> + broadcast per-room is the standard Tokio pattern for this use case; pattern verified from official Tokio docs
- Pitfalls: HIGH — RwLock across await, broadcast lag, and zone ID collision are well-documented and observed in practice; tutorial flag gap is design-specific but logically derived from D-09

**Research date:** 2026-03-24
**Valid until:** 2026-06-24 (90 days — toml, tokio, sqlx are stable; no fast-moving dependencies in Phase 2 scope)
