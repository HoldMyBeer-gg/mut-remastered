# Phase 03: Character and Combat - Context

**Gathered:** 2026-03-24
**Status:** Ready for planning

<domain>
## Phase Boundary

Players can create characters with race, class, ability scores, and gender; fight NPC monsters using D&D-lite dice mechanics; pick up, drop, and equip gear; and die without losing progress. This phase delivers AUTH-03 through AUTH-07, CHAR-01 through CHAR-05, and CMBT-01 through CMBT-06.

**Not in scope:** TUI rendering (Phase 4), chat channels (Phase 5), browser client (Phase 6), procedural dungeons (Phase 7). CHAR-01 (HP/mana/stamina always visible in TUI status bar) is partially addressed — the server tracks and sends vitals data, but the TUI rendering is Phase 4.

</domain>

<decisions>
## Implementation Decisions

### Combat Complexity (D-CC-01)
- **Decision:** Simple D&D-lite combat. d20+modifier attack rolls vs AC, damage dice, HP tracking. 4-6 classes with distinct feels but minimal special abilities. No spell slots, no saving throws, no multi-attack in v1.
- **Rationale:** Get combat working and feeling good first. Can deepen with class abilities in future phases. Matches MUT's social-first, combat-second philosophy.

### Playable Races (D-CC-02)
- **Decision:** Expanded 6-8 races: Human, Elf, Dwarf, Halfling, Orc, Gnome, Half-Elf, Tiefling. Each with a stat bonus and flavor text.
- **Rationale:** User wants more character variety. The stat bonus system is simple enough that balancing 8 races isn't significantly harder than 4.

### Phase Structure (D-CC-03)
- **Decision:** 3 plans: (1) Character creation + DB schema, (2) Combat engine + NPC spawns, (3) Inventory/equipment + integration tests.
- **Rationale:** Natural dependency order. Character data must exist before combat can reference it. Combat must work before loot drops make sense. Integration tests validate all 16 requirements at the end.

### Combat Tick Model (D-CC-04)
- **Decision:** Introduce a fixed-tick combat loop (~2 second rounds) as a separate Tokio task. This was deferred from Phase 2 (D-06) and is now needed for CMBT-02. The world remains event-driven for non-combat actions.
- **Rationale:** Combat needs timed rounds. A dedicated combat tick task processes queued actions per round without affecting the event-driven world commands.

### Character-Account Relationship (D-CC-05)
- **Decision:** Multiple characters per account (AUTH-03). Active character selected after login. Account owns characters; character owns inventory, stats, position.
- **Rationale:** Standard MUD pattern. Players want alts. Position tracking moves from account_id to character_id.

### Death Mechanic (D-CC-06)
- **Decision:** Soft death — player respawns at bind point with gear intact and an XP debt (CMBT-06). No corpse runs, no item loss.
- **Rationale:** Matches the social/exploration focus. Harsh death penalties drive players away from a social MUD.

### Vitals Delivery (D-CC-07)
- **Decision:** Server sends a Vitals message (HP, max_hp, mana, max_mana, stamina, max_stamina) after any action that changes them and on periodic heartbeat. The TUI will render these in Phase 4; for now the protocol delivers them.
- **Rationale:** CHAR-01 requires "always visible" vitals. The server must push them; the rendering is a client concern.

### Claude's Discretion
- Exact stat formulas (how ability scores map to modifiers, AC calculation)
- Class ability details and progression
- NPC data format (TOML like zones, or embedded in zone files)
- Monster stat blocks and loot table format
- Inventory slot names and equipment mechanics
- XP debt formula on death
- Combat message formatting
- Database schema for characters, inventory, combat state

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Server Architecture
- `server/src/session/actor.rs` — ConnectionActor with auth + world dispatch; must extend with character selection and combat commands
- `server/src/world/commands.rs` — World command handlers; combat commands follow same pattern
- `server/src/world/types.rs` — World, RoomId, RoomDef, RoomState; character and NPC types extend this or live in new modules
- `server/src/net/listener.rs` — AppState struct; will need combat state references
- `server/src/db.rs` — Database init with migrations

### Protocol
- `protocol/src/auth.rs` — ClientMsg/ServerMsg for auth; needs CharacterCreate, CharacterSelect, CharacterList variants
- `protocol/src/world.rs` — ClientMsg/ServerMsg for world; needs Attack, Flee, UseAbility, Vitals, CombatLog variants
- `protocol/src/codec.rs` — NS_AUTH, NS_WORLD namespaces; may need NS_COMBAT or extend NS_WORLD

### World Data
- `world/zones/` — Zone TOML files; NPC spawn definitions may go here or in separate monster data files
- `server/migrations/` — 4 existing migrations; new ones for characters, inventory, combat

### Planning
- `.planning/ROADMAP.md` — Phase 3 success criteria (5 must-be-true statements)
- `.planning/REQUIREMENTS.md` — AUTH-03-07, CHAR-01-05, CMBT-01-06 definitions
- `.planning/STATE.md` — Decisions from Phases 1-2 that constrain Phase 3

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ConnectionActor` pattern: Per-connection, auth-gated dispatch. Character commands extend `handle_world_message` or add a new `handle_combat_message` dispatcher.
- `World` struct: In-memory state with `Arc<RwLock<World>>`. Character/NPC data can be added as new fields.
- `broadcast::Sender` per room: Combat events (attack messages, death, loot) broadcast to the room the same way world events do.
- TestServer/TestClient helpers: Extend for character creation and combat test flows.
- TOML zone loader: Pattern for loading NPC spawn tables from data files.

### Established Patterns
- Actor-per-session with no shared mutable state (besides Arc-wrapped)
- postcard serialization for all protocol messages with namespace bytes
- sqlx runtime queries for DB operations
- Separate read/write lock phases to minimize lock contention
- Broadcast-after-write pattern (write to memory, persist to SQLite, then broadcast)

### Migration from account_id to character_id
- `player_positions` table currently uses `account_id`. After character creation, this should use `character_id` instead.
- In-memory `World::player_positions` needs the same migration: `HashMap<String, RoomId>` keyed by character_id rather than account_id.
- The `ConnectionActor` currently tracks `account_id`. It will also need an `active_character_id` field.

</code_context>

<specifics>
## Specific Ideas

- Romance-able quest NPCs (any gender ↔ any gender) — noted from user, deferred to future phase with quest/dialogue system
- Combat log should show dice roll results in plain language: "rolled 14 + 2 STR vs AC 12 — HIT! 7 damage" (from CMBT-01)
- Rounds should be ~2 seconds (from CMBT-02)

</specifics>

<deferred>
## Deferred Ideas

- Romance-able quest NPCs — requires dialogue/quest system not in Phase 3
- Spell slots and advanced class abilities — future combat deepening phase
- PvP combat — explicitly out of scope per REQUIREMENTS.md

</deferred>

---

*Phase: 03-character-and-combat*
*Context gathered: 2026-03-24*
