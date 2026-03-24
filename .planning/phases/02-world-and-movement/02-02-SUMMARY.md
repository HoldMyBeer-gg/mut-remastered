---
phase: 02-world-and-movement
plan: "02"
subsystem: world-runtime
tags: [rust, tokio, sqlx, sqlite, broadcast, world-commands, actor]

# Dependency graph
requires:
  - phase: 02-world-and-movement
    plan: "01"
    provides: World types, RoomDef, Direction, TriggerEffect, load_world, protocol::world messages, migrations 002+003

provides:
  - server/src/world/commands.rs — handle_look, handle_move, handle_examine, handle_interact
  - server/src/net/listener.rs — AppState with world: Arc<RwLock<World>> and room_channels: Arc<RwLock<HashMap<RoomId, broadcast::Sender<WorldEvent>>>>
  - server/src/session/actor.rs — ConnectionActor with tokio::select!, dispatch_frame, handle_world_message, room broadcast subscription
  - server/src/config.rs — worlds_dir field (MUT_WORLDS_DIR env var)
  - server/migrations/004_account_flags.sql — account_flags table for tutorial_complete and future feature flags
  - Direction::opposite() method in types.rs

affects:
  - 02-03 (newbie area and integration tests — can now write world command tests against this runtime)
  - 03-* (character system — player_positions already created; this plan wires it at login time)

# Tech tracking
tech-stack:
  added:
    - "tokio::sync::broadcast — per-room event broadcast channels (capacity 32)"
    - "tokio::select! — concurrent client frame + room event processing in ConnectionActor"
  patterns:
    - "Split-borrow pattern for tokio::select!: extract read_frame_from as free function to avoid E0500 borrow conflict"
    - "Dispatch-try pattern: decode_message::<auth::ClientMsg> first, then world::ClientMsg — avoids unified wrapper enum"
    - "Post-lock SQLite writes: acquire/drop RwLock before async DB calls to minimize lock hold time"
    - "Actor room subscription on login and re-subscription on every Move"

key-files:
  created:
    - "server/src/world/commands.rs — handle_look, handle_move, handle_examine, handle_interact"
    - "server/migrations/004_account_flags.sql — account_flags table"
  modified:
    - "server/src/net/listener.rs — AppState extended with world and room_channels"
    - "server/src/session/actor.rs — ConnectionActor with tokio::select!, world dispatch, login world placement"
    - "server/src/world/mod.rs — added pub mod commands"
    - "server/src/world/types.rs — added Direction::opposite(), removed unused FromStr import"
    - "server/src/config.rs — added worlds_dir field"
    - "server/src/main.rs — load_world at startup, broadcast channel creation, extended AppState construction"
    - "server/tests/helpers/mod.rs — AppState construction updated with empty world + room_channels"

key-decisions:
  - "Free function read_frame_from instead of self.read_frame(): Rust E0500 prevents holding &mut self.reader future and &mut self.room_receiver simultaneously in tokio::select! — extracting reader parameter as free function resolves the borrow conflict cleanly"
  - "Dispatch-try approach (Approach B) over unified ClientMessage wrapper: avoids breaking existing auth integration tests; postcard enum variant indices make cross-decode false positives extremely unlikely"
  - "Empty World in test helpers: integration tests exercise auth, not world commands; providing an empty default World satisfies AppState construction without needing zone file I/O in tests"
  - "DEFAULT_SPAWN_ROOM = starting_village:market_square: tutorial area from Plan 01 zone TOML; first-time players placed here on login per D-07/D-08"

# Metrics
duration: 4min
completed: 2026-03-24
---

# Phase 02 Plan 02: World Runtime Wiring — Movement, Look, Examine, Interact

**In-memory world wired into the server: AppState carries Arc<RwLock<World>>, ConnectionActor dispatches world commands via tokio::select!, players are placed in the world on login and can move between rooms with broadcast to room peers**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-24T01:31:32Z
- **Completed:** 2026-03-24T01:35:39Z
- **Tasks:** 2
- **Files modified:** 9 (1 created new, 1 migration, 7 modified)

## Accomplishments

- World command handlers in commands.rs: handle_look (RoomDescription with merged exits, players_here, conditional hints), handle_move (position update + SQLite persist + room broadcast + auto-look), handle_examine (lore text or "nothing of note"), handle_interact (generic trigger evaluation with SetState/RevealExit/Message/Broadcast/SetTutorialComplete effects)
- AppState extended with world: Arc<RwLock<World>> and room_channels: Arc<RwLock<HashMap<RoomId, broadcast::Sender<WorldEvent>>>> — both fields shared across all connections
- main.rs updated to load_world at startup and create per-room broadcast channels before building AppState
- ConnectionActor refactored with tokio::select! loop — client frames and room broadcast events processed concurrently
- dispatch_frame tries auth::ClientMsg first then world::ClientMsg — existing auth flow untouched
- Login now: ensures player has a room position (default starting_village:market_square), subscribes to room broadcast channel, loads tutorial_complete flag from account_flags table
- Move handler re-subscribes actor to new room's channel after position update
- Interact handler returns (Vec<ServerMsg>, bool) — bool signals tutorial completion to actor
- account_flags table migration (004) for tutorial_complete and future per-account feature flags
- All 8 existing integration tests pass without modification

## Task Commits

1. **Task 1: AppState extension, world commands module, and main.rs world loading** - `1500c39` (feat)
2. **Task 2: ConnectionActor world command dispatch with tokio::select! and room broadcast** - `3649cf8` (feat)

## Files Created/Modified

- `server/src/world/commands.rs` — handle_look, handle_move, handle_examine, handle_interact (created)
- `server/migrations/004_account_flags.sql` — account_flags table (created)
- `server/src/net/listener.rs` — AppState with world + room_channels fields
- `server/src/session/actor.rs` — ConnectionActor with tokio::select!, dispatch_frame, handle_world_message, login world placement
- `server/src/world/mod.rs` — added pub mod commands
- `server/src/world/types.rs` — Direction::opposite(), removed unused import
- `server/src/config.rs` — worlds_dir field
- `server/src/main.rs` — load_world at startup, broadcast channel loop, extended AppState
- `server/tests/helpers/mod.rs` — AppState updated with empty world + room_channels

## Decisions Made

- Free function `read_frame_from` instead of `self.read_frame()` method: Rust E0500 prevents the borrow checker from allowing `self.read_frame()` future and `&mut self.room_receiver` to coexist in the same `tokio::select!` — making it a free function that takes `&mut OwnedReadHalf` resolves this cleanly
- Dispatch-try approach over unified ClientMessage wrapper enum: Approach B (try auth first, then world) avoids modifying or rewriting existing auth integration tests; postcard uses deterministic enum variant indices making cross-type false positives extremely unlikely in practice
- Empty default World in test helpers: auth integration tests don't exercise world commands; providing `World::default()` and an empty `HashMap` for room_channels satisfies the AppState struct without loading zone TOML files during test setup
- `starting_village:market_square` as default spawn: matches the zone TOML created in Plan 01, implements D-07/D-08 (new players start in the tutorial/starting area)

## Deviations from Plan

**1. [Rule 1 - Bug] Free function for read_frame to resolve E0500 borrow conflict**
- **Found during:** Task 2
- **Issue:** `tokio::select!` macro creates futures for both branches simultaneously; holding `self.read_frame()` future (borrows `self`) and `&mut self.room_receiver` (also borrows `self`) causes E0500 borrow conflict
- **Fix:** Extracted `read_frame_from(reader: &mut OwnedReadHalf)` as a module-level free function; the select! branches now borrow `self.reader` and `self.room_receiver` independently
- **Files modified:** `server/src/session/actor.rs`
- **Commit:** 3649cf8

**2. [Rule 2 - Missing functionality] Updated test helpers to construct extended AppState**
- **Found during:** Task 2 (cargo test run)
- **Issue:** Integration test helpers constructed `AppState { db, session_ttl_secs }` — missing the two new fields caused compile errors in all integration tests
- **Fix:** Updated `server/tests/helpers/mod.rs` to add `Arc::new(RwLock::new(World::default()))` and empty `HashMap` for room_channels
- **Files modified:** `server/tests/helpers/mod.rs`
- **Commit:** 3649cf8

## Known Stubs

None — all command handlers return real data from in-memory World. The world has content from Plan 01's zone TOML (starting_village). Plan 03 adds more zones and integration tests to verify end-to-end behavior.

## Self-Check: PASSED

Files verified present:
- FOUND: server/src/world/commands.rs
- FOUND: server/migrations/004_account_flags.sql
- FOUND: server/src/net/listener.rs (contains Arc<RwLock<World>>)
- FOUND: server/src/session/actor.rs (contains tokio::select!)
- FOUND: server/src/main.rs (contains load_world)

Commits verified:
- FOUND: 1500c39 (Task 1)
- FOUND: 3649cf8 (Task 2)

Tests: 8/8 passing (7 auth_integration + 1 concurrent_connections)

---
*Phase: 02-world-and-movement*
*Completed: 2026-03-24*
