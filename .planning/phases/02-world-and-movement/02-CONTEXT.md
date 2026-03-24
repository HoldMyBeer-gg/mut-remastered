# Phase 02: World and Movement - Context

**Gathered:** 2026-03-24
**Status:** Ready for planning

<domain>
## Phase Boundary

Players can explore a persistent hand-crafted world with rooms, movement commands, lore, a newbie tutorial area, and interactive objects that change world state. This phase delivers WRLD-01 through WRLD-06.

**Not in scope:** Combat, character creation, chat channels, TUI rendering — those are Phases 3-5.

</domain>

<decisions>
## Implementation Decisions

### Room Data Format
- **D-01:** Rooms are defined as TOML data files loaded at startup (one file per zone or region). Room definitions include descriptions, exits, lore text, and trigger definitions. This keeps world authoring in version-controllable text files separate from code.
- **D-02:** Runtime state changes (e.g., a lever pulled, a blockage cleared) are persisted in SQLite. On startup, the server loads TOML room definitions and then overlays any persisted state mutations from the database.
- **D-03:** Player positions are stored in SQLite (account_id → room_id mapping) to satisfy WRLD-03 (survive server restart).

### World Tick Model
- **D-04:** Phase 2 uses event-driven command processing — player types a command, server processes it immediately, sends response. No fixed-tick loop yet.
- **D-05:** The world state (rooms, player positions, object states) lives in an in-memory data structure (e.g., `Arc<RwLock<World>>` or similar) that is the single source of truth during runtime. SQLite is the persistence layer, not the query layer for hot-path operations.
- **D-06:** Fixed-tick ECS can be introduced in Phase 3 when combat round timers require it. Phase 2 does not need time-based systems.

### Newbie Area Design
- **D-07:** The newbie area is a freely explorable zone (not a forced linear corridor). Players receive contextual hints when they enter rooms or attempt commands — e.g., "Try typing 'north' to move through the archway" when they enter a room with a north exit.
- **D-08:** The newbie area has 5-8 rooms that naturally teach movement, looking, examining objects, and interacting with triggers before leading to the wider world.
- **D-09:** Hints are stored as part of the room data (a `hints` field in the TOML). The server sends hints only to players who haven't completed the tutorial flag.

### Interactable Objects and State Mutations
- **D-10:** Room TOML files define triggers as data: a trigger has an activation command (e.g., "pull lever"), a condition (optional, e.g., `lever_state == "unpulled"`), and effects (e.g., set `lever_state = "pulled"`, reveal exit to east, broadcast message to room).
- **D-11:** A generic trigger handler in the server processes these data-driven triggers. No per-room hardcoded logic — all behavior comes from the data files.
- **D-12:** Trigger state changes are broadcast to all players in the room (satisfies WRLD-06: "reflected for all players in that area").

### Claude's Discretion
- Room TOML file structure and field names
- How zones/regions are organized (single file per zone, directory structure)
- Internal data structures for the world state
- Protocol message types for movement, room descriptions, and world events
- Migration schema for world state persistence tables

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Server Architecture
- `server/src/session/actor.rs` — Current connection actor pattern; new commands (movement, look, interact) extend the message dispatch here
- `server/src/net/listener.rs` — AppState struct that will need to include World reference
- `server/src/db.rs` — Database init pattern; new migrations for world state tables follow this

### Protocol
- `protocol/src/auth.rs` — Existing message types; world commands need new ClientMsg/ServerMsg variants
- `protocol/src/codec.rs` — Framing layer; no changes expected but must understand the format

### Project Context
- `.planning/PROJECT.md` — Core value: "social interaction and exploration come first"
- `.planning/REQUIREMENTS.md` — WRLD-01 through WRLD-06 definitions
- `.planning/phases/01-server-foundation/01-RESEARCH.md` — Phase 1 research with architecture decisions

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ConnectionActor` (session/actor.rs): Per-connection actor pattern — world commands will extend `handle_message` with new `ClientMsg` variants
- `AppState` (net/listener.rs): Currently holds `db: SqlitePool` and `session_ttl_secs` — needs a `world: Arc<World>` or similar shared reference
- `protocol::codec`: Length-prefixed encode/decode — works for any new message types
- `server::db::init_db`: Migration runner — new world tables follow the same migration pattern

### Established Patterns
- Actor-per-session with no shared mutable state (besides SqlitePool)
- postcard serialization for all protocol messages
- sqlx runtime queries (not compile-time macros) for flexibility
- Integration test pattern with TestServer/TestClient helpers

### Integration Points
- `ClientMsg` enum needs world command variants (Move, Look, Examine, Interact)
- `ServerMsg` enum needs room/world response variants (RoomDescription, MoveOk, WorldEvent)
- `AppState` needs world reference for the actor to query room data
- `server/src/main.rs` startup needs world loading before listener start
- New migration files for player_positions and world_state tables

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches for MUD room systems. The D&D flavor should come through in room descriptions and lore text.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 02-world-and-movement*
*Context gathered: 2026-03-24*
