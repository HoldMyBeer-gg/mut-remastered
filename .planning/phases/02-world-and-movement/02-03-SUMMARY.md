---
phase: 02-world-and-movement
plan: "03"
subsystem: world-testing
tags: [rust, tokio, sqlx, sqlite, toml, integration-tests, protocol, world-commands]

# Dependency graph
requires:
  - phase: 02-world-and-movement
    plan: "02"
    provides: AppState with world+room_channels, handle_look/move/examine/interact, actor dispatch

provides:
  - world/zones/newbie/zone.toml — 7-room Warden's Academy tutorial zone
  - server/tests/world_integration.rs — 11 integration tests covering WRLD-01 through WRLD-06
  - server/tests/helpers/mod.rs — extended TestServer and TestClient with world command support
  - protocol/src/codec.rs — namespace-byte wire protocol (NS_AUTH=0x01, NS_WORLD=0x02)

affects:
  - All future phases that use encode_message/decode_message (now require namespace param)
  - 03-* (character system — world runtime proven correct by test suite)

# Tech tracking
tech-stack:
  added:
    - "tempfile 3 (dev-dep) — file-based SQLite temp files for persistence tests"
    - "uuid 1 (dev-dep in server) — unique usernames in parallel integration tests"
  patterns:
    - "NS_AUTH/NS_WORLD namespace byte prefix in every wire frame — prevents cross-decode false positives between auth and world message types"
    - "TestServer::start_with_spawn(room_id) — overrides World::default_spawn for test isolation without recompile"
    - "World::default_spawn: Option<RoomId> field — runtime-configurable spawn point for testing"
    - "CARGO_MANIFEST_DIR for zone path resolution — resolves correctly regardless of working directory"
    - "Absent state key treated as 'false' in TriggerCondition check — allows initial trigger fires without pre-seeding DB"

key-files:
  created:
    - "world/zones/newbie/zone.toml — 7-room Warden's Academy with hints, lore, triggers, SetTutorialComplete"
    - "server/tests/world_integration.rs — 11 WRLD integration tests"
  modified:
    - "server/tests/helpers/mod.rs — extended with world loading, custom spawn, world command helpers"
    - "protocol/src/codec.rs — added NS_AUTH/NS_WORLD namespace byte to wire protocol"
    - "server/src/session/actor.rs — dispatch uses namespace-aware decode; spawn override support"
    - "server/src/world/types.rs — added World::default_spawn field"
    - "server/src/world/commands.rs — fixed TriggerCondition absent-key treated as 'false'"
    - "server/src/world/loader.rs — pass default_spawn: None in World construction"
    - "server/Cargo.toml — added tempfile and uuid dev-dependencies"
    - "client-tui/src/main.rs — updated to new encode_message/decode_message signatures"

key-decisions:
  - "Namespace byte (NS_AUTH/NS_WORLD) added to wire protocol: postcard from_bytes is lenient — unit-variant auth messages (Logout, Ping at indices 2/3) silently matched data-carrying world messages (Examine, Interact at same indices). Namespace byte is the minimal correct fix."
  - "TriggerCondition absent-key defaults to 'false': allows TOML conditions like {key='lever_state', value='false'} to fire on fresh rooms without requiring DB pre-seeding"
  - "World::default_spawn field for test-time spawn override: avoids needing to recompile DEFAULT_SPAWN_ROOM constant; test helpers set this before starting the server"
  - "TestServer::start() uses empty world for auth tests (preserving existing test behavior); start_with_world/start_with_spawn load real zone TOML files"

patterns-established:
  - "Wire frame format: [u32 LE len][u8 namespace][postcard payload] — namespace distinguishes auth (0x01) from world (0x02)"
  - "Zone TOML lever pattern: first trigger has condition {key='lever_state', value='false'}, second has condition {key='lever_state', value='true'} — bidirectional state transitions"
  - "Integration test helpers: register_and_login convenience method; recv_world drain pattern after broadcast triggers"

requirements-completed: [WRLD-01, WRLD-02, WRLD-03, WRLD-04, WRLD-05, WRLD-06]

# Metrics
duration: 11min
completed: 2026-03-24
---

# Phase 02 Plan 03: Newbie Zone, Test Helpers, and WRLD Integration Tests

**7-room tutorial zone, 11 WRLD integration tests, and a wire protocol namespace fix that eliminates auth/world cross-decode false positives**

## Performance

- **Duration:** 11 min
- **Started:** 2026-03-24T01:37:58Z
- **Completed:** 2026-03-24T01:49:48Z
- **Tasks:** 2
- **Files modified:** 9 (2 created, 7 modified)

## Accomplishments

- Warden's Academy 7-room tutorial zone with D&D-flavored prose: entrance, notice_board, courtyard, practice_hall, hidden_chamber, armory, garden — 7 lore entries, 6 hint arrays, 8 triggers including conditions, set_state, reveal_exit, broadcast, and SetTutorialComplete
- 11 integration tests covering all WRLD requirements end-to-end:
  - WRLD-01 (look), WRLD-02 (move/alias/no-exit), WRLD-03 (persistence across restart), WRLD-04 (hints shown/suppressed), WRLD-05 (examine lore/unknown), WRLD-06 (trigger fires+persists, broadcast to room)
- TestServer extended with start_with_world, start_with_db, start_with_spawn variants; TestClient extended with register_and_login, send_world, recv_world, send_move, send_look, send_examine, send_interact
- Wire protocol namespace byte added to eliminate postcard cross-decode false positives

## Task Commits

1. **Task 1: Newbie tutorial zone TOML** - `bcfe0e1` (feat)
2. **Task 2: Test helpers extension and WRLD integration tests** - `0509860` (feat)

## Files Created/Modified

- `world/zones/newbie/zone.toml` — 7-room Warden's Academy zone (created)
- `server/tests/world_integration.rs` — 11 WRLD integration tests (created)
- `server/tests/helpers/mod.rs` — extended TestServer/TestClient for world commands
- `protocol/src/codec.rs` — namespace-byte wire protocol
- `server/src/session/actor.rs` — namespace-aware dispatch, spawn override
- `server/src/world/types.rs` — World::default_spawn field
- `server/src/world/commands.rs` — TriggerCondition absent-key = "false" fix
- `server/src/world/loader.rs` — default_spawn: None in World construction
- `server/Cargo.toml` — tempfile + uuid dev-dependencies
- `client-tui/src/main.rs` — updated codec call signatures

## Decisions Made

- **Namespace byte in wire protocol (Rule 1 - Bug):** postcard's `from_bytes` is lenient — it does not verify remaining bytes. `world::Examine { target }` (variant 2) cross-decoded as `auth::Logout` (unit variant 2), causing the server to process examine commands as logouts. Added `NS_AUTH = 0x01` and `NS_WORLD = 0x02` namespace byte to every encoded frame. All callers updated — no silent cross-decode possible.
- **TriggerCondition absent-key = "false" (Rule 1 - Bug):** condition check returned `false` when room_state was `None`, preventing first-time trigger fires for conditions like `{ key = "lever_state", value = "false" }`. Fixed by defaulting absent key to `"false"` — the conventional initial state for boolean trigger conditions.
- **World::default_spawn field for test spawn override:** Rather than patching `DEFAULT_SPAWN_ROOM` const (requires recompile), added `default_spawn: Option<RoomId>` to World struct. Test helpers set this field before starting the server task.

## Deviations from Plan

**1. [Rule 1 - Bug] Namespace byte added to wire protocol to fix cross-decode false positives**
- **Found during:** Task 2 (test execution)
- **Issue:** `world::Examine { target: "room" }` (postcard variant 2) cross-decoded as `auth::Logout` (unit variant 2) because postcard's `from_bytes` ignores trailing bytes. Server processed Examine as Logout, sending `LogoutOk` instead of `ExamineResult`.
- **Fix:** Added 1-byte namespace discriminant (`NS_AUTH=0x01`, `NS_WORLD=0x02`) to `encode_message`/`decode_message` signatures. All callers updated.
- **Files modified:** `protocol/src/codec.rs`, `server/src/session/actor.rs`, `server/tests/helpers/mod.rs`, `client-tui/src/main.rs`
- **Commits:** 0509860

**2. [Rule 1 - Bug] TriggerCondition absent-key defaulted to "false" instead of returning false**
- **Found during:** Task 1 analysis / Task 2 test design
- **Issue:** `dispatch_frame` condition check returned `false` when no room state existed, preventing initial trigger fires (e.g., first `pull lever` on a fresh room with no state).
- **Fix:** When key is absent from room KV store, treat as `"false"` — the conventional initial value for boolean trigger conditions.
- **Files modified:** `server/src/world/commands.rs`
- **Commits:** 0509860

**3. [Rule 2 - Missing functionality] World::default_spawn field for test-time spawn override**
- **Found during:** Task 2 (implementing test_hints_suppressed_after_tutorial and test_trigger_fires_and_persists)
- **Issue:** Tests needed players to spawn in specific rooms (newbie:entrance, newbie:practice_hall) but DEFAULT_SPAWN_ROOM was a compiled constant. No runtime override existed.
- **Fix:** Added `default_spawn: Option<RoomId>` to World struct. Actor checks this before falling back to the constant. TestServer constructors can set this field.
- **Files modified:** `server/src/world/types.rs`, `server/src/world/loader.rs`, `server/src/session/actor.rs`
- **Commits:** 0509860

## Known Stubs

None — all 11 tests exercise real command handlers with real zone data.

## Self-Check: PASSED

Files verified present:
- FOUND: world/zones/newbie/zone.toml (7 rooms, 8 triggers)
- FOUND: server/tests/world_integration.rs (11 tests)
- FOUND: server/tests/helpers/mod.rs (contains start_with_world, send_move)

Commits verified:
- FOUND: bcfe0e1 (Task 1)
- FOUND: 0509860 (Task 2)

Tests: 19/19 passing (7 auth_integration + 1 concurrent_connections + 11 world_integration)

---
*Phase: 02-world-and-movement*
*Completed: 2026-03-24*
