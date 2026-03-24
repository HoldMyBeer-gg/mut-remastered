---
phase: 02-world-and-movement
plan: "01"
subsystem: world
tags: [rust, toml, sqlx, sqlite, serde, protocol, world-state]

# Dependency graph
requires:
  - phase: 01-server-foundation
    provides: SQLite pool, AppState, protocol crate structure, migration runner, postcard codec

provides:
  - protocol/src/world.rs — ClientMsg (Move, Look, Examine, Interact) and ServerMsg (RoomDescription, MoveOk, MoveFail, ExamineResult, InteractResult, WorldEvent)
  - server/src/world/types.rs — RoomId, Direction, RoomDef, TriggerDef, TriggerCondition, TriggerEffect, RoomState, World, WorldEvent types
  - server/src/world/loader.rs — load_world function that reads zone TOML subdirectories and overlays SQLite state
  - world/zones/starting_village/zone.toml — 4-room Briarhollow Village zone with lore, hints, and data-driven triggers
  - SQLite migrations 002_player_positions and 003_world_state
  - toml 1.1 dependency added to server

affects:
  - 02-02 (movement runtime — wires load_world into AppState, implements command handlers)
  - 02-03 (newbie area and tests — adds more zone content, tests against these types)
  - 03-* (character system — player_positions table already created)

# Tech tracking
tech-stack:
  added:
    - "toml 1.1 — zone TOML parsing via serde Deserialize"
  patterns:
    - "Separate protocol message enums per domain (auth vs world) — both in protocol crate, routed by server"
    - "RoomId newtype (String) — prevents accidental raw string comparisons, derives Hash+Eq for HashMap keys"
    - "Data-driven trigger system — TOML defines command+condition+effects, generic handler evaluates at runtime"
    - "Loader reads zone TOML files from directory, validates no room ID collisions, then overlays SQLite state"

key-files:
  created:
    - "protocol/src/world.rs — world protocol message types"
    - "server/src/world/types.rs — all world data structures"
    - "server/src/world/mod.rs — module declaration"
    - "server/src/world/loader.rs — load_world async function"
    - "world/zones/starting_village/zone.toml — starting village 4-room zone"
    - "server/migrations/002_player_positions.sql — player_positions table"
    - "server/migrations/003_world_state.sql — world_state table"
  modified:
    - "protocol/src/lib.rs — added pub mod world"
    - "server/src/lib.rs — added pub mod world"
    - "server/Cargo.toml — added toml = \"1.1\""

key-decisions:
  - "Loader written in full in Task 1 (not a stub) — mod.rs references loader so it must compile; writing it complete avoided two-step approach"
  - "ZoneFile struct (zone_id, zone_name, rooms) is private to loader — only World is returned to callers"
  - "Direction::from_str is inherent method (not std::str::FromStr trait) to return Option<Self> without error type"
  - "TriggerEffect uses serde tag = kind + rename_all = snake_case to match TOML {kind = set_state, ...} syntax"

patterns-established:
  - "Zone TOML structure: zone_id, zone_name, [[rooms]] with id/name/description/lore/hints/exits/[[rooms.triggers]]"
  - "Trigger effects as inline tables: { kind = \"message\", text = \"...\" }"
  - "Room IDs are zone-prefixed strings: zone_id:room_slug (e.g., starting_village:market_square)"

requirements-completed: [WRLD-01, WRLD-02, WRLD-05, WRLD-06]

# Metrics
duration: 2min
completed: 2026-03-24
---

# Phase 02 Plan 01: World Data Contracts, Types, and Zone TOML

**TOML-based world data layer with protocol message types, RoomId/Direction/TriggerEffect types, SQLite migrations, and a 4-room Briarhollow Village zone with data-driven triggers**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-24T01:26:51Z
- **Completed:** 2026-03-24T01:29:11Z
- **Tasks:** 2
- **Files modified:** 10 (7 created, 3 modified)

## Accomplishments

- World protocol messages defined in protocol crate — ClientMsg (Move, Look, Examine, Interact) and ServerMsg (RoomDescription, MoveOk, MoveFail, ExamineResult, InteractResult, WorldEvent)
- Complete world type system in server — RoomId newtype, Direction enum with aliases, RoomDef/TriggerDef/TriggerEffect for TOML parsing, RoomState/World for runtime state
- load_world async function that reads zone TOML subdirectory files, validates room ID collisions, and overlays persisted SQLite state from player_positions and world_state tables
- Briarhollow Village 4-room starting zone with lore, tutorial hints, and data-driven triggers (examine well, examine map, pull lever with condition+set_state+broadcast, examine grate)

## Task Commits

1. **Task 1: Protocol world messages, world types, and migrations** - `6f5927d` (feat)
2. **Task 2: World loader and starting village zone TOML** - `a86074f` (feat)

## Files Created/Modified

- `protocol/src/world.rs` — World-domain ClientMsg and ServerMsg enums (serializable with postcard)
- `protocol/src/lib.rs` — Added `pub mod world`
- `server/src/world/types.rs` — RoomId, Direction, RoomDef, TriggerDef, TriggerCondition, TriggerEffect, RoomState, World, WorldEvent
- `server/src/world/mod.rs` — Module declarations for types and loader
- `server/src/world/loader.rs` — load_world: reads zones_dir subdirectories, parses TOML, validates uniqueness, overlays SQLite state
- `server/src/lib.rs` — Added `pub mod world`
- `server/Cargo.toml` — Added `toml = "1.1"`
- `server/migrations/002_player_positions.sql` — player_positions table (account_id PK, room_id, updated_at)
- `server/migrations/003_world_state.sql` — world_state table (room_id + state_key composite PK, state_value, updated_at)
- `world/zones/starting_village/zone.toml` — 4-room Briarhollow Village zone with triggers

## Decisions Made

- Loader fully implemented in Task 1 rather than stub — mod.rs references loader module so it must compile cleanly; writing complete implementation upfront was cleaner than a two-step approach
- Direction uses an inherent `from_str` method returning `Option<Self>` rather than implementing `std::str::FromStr` to avoid the required error type
- TriggerEffect uses `#[serde(tag = "kind", rename_all = "snake_case")]` to match the TOML inline-table syntax `{ kind = "set_state", key = "...", value = "..." }`
- ZoneFile struct kept private to loader — only the assembled World is exposed to callers

## Deviations from Plan

None — plan executed exactly as written. The loader was written in full during Task 1 (rather than as a stub with Task 2 filling it in), but this was a sequencing efficiency, not a deviation from the plan's stated acceptance criteria.

## Issues Encountered

None.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plan 02 (movement runtime) can now wire `load_world` into `AppState`, add `Arc<RwLock<World>>`, and implement Move/Look/Examine/Interact command handlers against these types
- Plan 03 (newbie area and tests) can write integration tests against the zone TOML and these types
- Migration runner in server/src/db.rs will apply migrations 002 and 003 automatically on next startup

## Self-Check: PASSED

All files verified present and commits confirmed in git history:
- FOUND: protocol/src/world.rs
- FOUND: server/src/world/types.rs
- FOUND: server/src/world/loader.rs
- FOUND: world/zones/starting_village/zone.toml
- FOUND: server/migrations/002_player_positions.sql
- FOUND: server/migrations/003_world_state.sql
- FOUND: commit 6f5927d (Task 1)
- FOUND: commit a86074f (Task 2)

---
*Phase: 02-world-and-movement*
*Completed: 2026-03-24*
