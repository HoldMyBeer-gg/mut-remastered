# Project Research Summary

**Project:** MUT Remastered
**Domain:** Multi-User Text Dungeon (MUD) — Rust game server, dual TUI clients (native + browser), web character armory
**Researched:** 2026-03-23
**Confidence:** HIGH (stack), MEDIUM (features, architecture), MEDIUM (pitfalls)

## Executive Summary

MUT Remastered is a modern reimagining of classic MUD games built on a Rust-first stack with two clients: a native Ratatui TUI and a browser-based xterm.js terminal. This project sits in an unusual position — the MUD genre is well-understood but modern Rust implementations are sparse, so architecture patterns are synthesized from Tokio game server community patterns rather than direct prior art. The recommended approach is a single authoritative Tokio process with an actor-per-session model, fixed-tick ECS world loop, and a protocol-first shared crate that both clients compile against. Start with SQLite and hand-crafted world content; layer procedural generation and the web armory once the core social + exploration loop is validated.

The clearest competitive differentiators are features no MUD has today: a structured Ratatui TUI with persistent split panels, a browser client that works without installing any software, and a web character armory with 3D gear visualization. These require more upfront investment than a raw telnet server but are the product's entire reason to exist alongside existing MUDs. The feature research confirms that these differentiators are additive to — not replacements for — the genre table stakes (persistent world, room system, basic combat, chat channels). Build the genre foundations first, then layer the modern presentation on top.

The dominant risks are architectural, not feature-level. Three pitfalls are high-recovery-cost and must be prevented before any gameplay is written: a monolithic game loop that serializes all players, shared mutable world state behind a global `Mutex`, and D&D rules scope creep that creates an unbounded combat implementation surface. All three are explicitly called out in PITFALLS.md as "must address in Phase 1" issues. A fourth structural risk — dual-client protocol drift — must be prevented in the protocol definition phase before either client is built. Everything else (xterm.js flow control, Unicode width constraints, WebSocket reconnect) is low-to-medium recovery cost and can be addressed when the relevant component is built.

## Key Findings

### Recommended Stack

The server is Rust on Tokio 1.48 with Axum 0.8 handling both WebSocket and HTTP REST, SQLx 0.8 for async database access (SQLite in dev, PostgreSQL in production), and the `tracing` crate for structured async logging. The native TUI client uses Ratatui 0.30 + Crossterm 0.29. The web client is xterm.js 6.0 connected to the server via `@xterm/addon-attach` over WebSocket. The web armory is SvelteKit 2.x with Threlte 8 for 3D character model rendering. There is no need for a separate real-time framework — Tokio's in-process broadcast channels handle all fan-out for a single-server deployment. See `.planning/research/STACK.md` for full version compatibility table.

**Core technologies:**
- Tokio 1.48: async runtime — de facto standard; all major crates (Axum, SQLx, tracing) assume it
- Axum 0.8: HTTP + WebSocket server — native `ws` upgrade eliminates a separate WebSocket layer
- SQLx 0.8: database access — compile-time checked SQL, same code for SQLite dev and PostgreSQL prod
- Ratatui 0.30 + Crossterm 0.29: native TUI — actively maintained; immediate-mode rendering suits game UIs
- xterm.js 6.0 + @xterm/addon-attach: browser TUI — industry standard; renders ANSI from server directly
- SvelteKit 2 + Threlte 8: web armory — smaller bundles than React; clean Three.js integration for 3D gear
- hecs (ECS): entity-component system — lightweight, no scheduler, fits a manually-driven tick loop
- argon2 0.5: password hashing — OWASP 2025 first choice; memory-hard

**Critical version notes:**
- Axum 0.8 upgraded to hyper 1.0; do not mix with crates depending on hyper 0.x
- Threlte 8 requires Svelte 5 — verify before scaffolding
- `@xterm/xterm` 6.0 is scoped package; the old `xterm` npm package is obsolete

### Expected Features

The genre table stakes are well-established. The differentiators are clear and achievable. Anti-features (permadeath, full D&D 5e rules, PvP ganking, real-time combat) are explicitly out of scope and should be defended against during requirements. See `.planning/research/FEATURES.md` for full prioritization matrix and dependency graph.

**Must have (table stakes — P1):**
- Account creation + login — root dependency for everything else
- Character creation: name, race (3-4), class (4), ability scores
- Room system with descriptions + exits, movement commands (n/s/e/w/u/d)
- Persistent world with hand-crafted starting zone (50-100 rooms)
- HP/mana/stamina display always visible
- Local chat (say, emote, whisper) + global gossip channel
- Basic combat: attack roll vs AC, damage, flee, death + bind-point respawn
- NPC monsters with simple loot tables
- Inventory + body-slot equipment system
- Ratatui TUI: split panels (room, vitals bar, chat, input)
- Native terminal client + xterm.js browser client
- Player-to-player inspection, help system, newbie area

**Should have (competitive differentiators — P2):**
- Rich Unicode TUI with compass/minimap overlay
- xterm.js browser client (no download required) — barrier removal
- Web character armory with public profiles
- 3D gear visualization via blend-ai models in armory
- Procedural dungeons with hand-crafted set pieces
- D&D dice roll display in combat log ("rolled 17 + 3 vs AC 14 — HIT!")
- Character biography field

**Defer (v2+):**
- Player housing, trading/auction house, crafting system
- Consensual PvP dueling arena
- Guild/clan system

### Architecture Approach

The recommended architecture is a Tokio actor-per-session model with a central World Actor running a fixed tick loop (5-20 Hz) that owns an ECS world. Session actors handle I/O per-connection; they send typed commands upward and receive typed events back. A shared protocol crate defines `ClientMessage` / `ServerMessage` enums that both the native client and server compile against — this is the structural guarantee against client drift. The web armory is deliberately decoupled from the game engine: it reads PostgreSQL directly through Axum REST routes, with no involvement from the game tick loop. The dungeon generator lives in an independent crate with no async dependencies, callable via `spawn_blocking`. See `.planning/research/ARCHITECTURE.md` for the full system diagram, data flow diagrams, and anti-pattern analysis.

**Major components:**
1. Session Actor (per connection) — owns raw I/O; parses to typed commands; isolated from world state
2. World Actor (tick loop) — authoritative ECS owner; drains command queue each tick; emits events
3. Protocol Handler — normalizes TCP + WebSocket transports into uniform PlayerInput before anything reaches the game engine
4. Chat System — room-scoped mpsc for local chat; tokio broadcast for global; Redis upgrade path
5. Dungeon Generator (separate crate) — BSP + cellular automata; set piece injection; no async deps
6. Armory REST API (Axum) — reads PostgreSQL directly; no game engine coupling; separate from game WebSocket
7. Persistence (SQLx) — write only on events (login, logout, level up, zone change); never per-tick

### Critical Pitfalls

See `.planning/research/PITFALLS.md` for full detail on all 11 pitfalls, recovery costs, and phase-to-prevention mapping.

1. **Monolithic game loop** — Design three separate stages from day one: (1) command ingestion per session, (2) world simulation on fixed tick, (3) event fan-out. Recovery cost is HIGH if this is refactored later.
2. **Shared mutable world state (`Arc<Mutex<World>>`)** — Use ECS with ID-based entity relations from the start; no direct cross-entity references. Recovery cost is HIGH (full ECS refactor).
3. **D&D rules scope creep** — Define the MUT Remastered subset explicitly before any combat code: 6 ability scores, AC, HP, attack roll, saving throw. Cut concentration, opportunity attacks, action economy. Write it as a document first. Recovery cost is HIGH.
4. **Blocking code on Tokio workers** — Use `tokio::task::spawn_blocking` for dungeon generation, pathfinding, and any CPU-bound work. Install `tokio-blocked` in dev to catch regressions. Recovery cost is MEDIUM.
5. **xterm.js flow control ignored** — Implement watermark-based ACK flow control before first web client demo; WebSocket is not an unlimited pipe. Silent data truncation looks like rendering bugs. Recovery cost is LOW but invisible until production.

## Implications for Roadmap

The architecture's build-order graph and the pitfall-to-phase mapping converge on the same ordering. The FEATURES.md dependency graph confirms: account/login is the root dependency, the room system is the world primitive, and the web armory is deliberately decoupled and can be deferred.

### Phase 1: Server Foundation
**Rationale:** Three high-recovery-cost pitfalls (monolithic loop, shared mutex world state, blocking on Tokio workers) must be prevented before any gameplay is written. The shared protocol crate and ECS world model are structural decisions that all subsequent phases compile against. This is not optional scaffolding — it is the architectural spine.
**Delivers:** Working Tokio server with actor-per-session pattern, fixed-tick World Actor, hecs ECS world stub, SQLx schema + migrations, session auth (argon2 password hashing, JWT for web armory), shared protocol crate compiling against both server and native client skeleton.
**Addresses:** Account/login system (table stakes root dependency)
**Avoids:** Pitfalls 1 (monolithic loop), 2 (shared mutex), 4 (blocking Tokio workers), 6 (mixed persistence state), 10 (dual-client protocol drift)

### Phase 2: World and Movement
**Rationale:** Room system is the dependency for movement, chat scoping, minimap, combat placement, and dungeon generation. Must exist before any other gameplay. Hand-crafted starting zone validates the exploration loop before procedural generation is built.
**Delivers:** Room model (data-driven, not hard-coded), movement commands (n/s/e/w/u/d + aliases), room descriptions + exits, `look` command, persistent world state (rooms survive reboots), newbie area (50-100 hand-crafted rooms), help system.
**Addresses:** Room system + movement (P1), persistent world (P1), newbie area (P1), help system (P1)
**Avoids:** Pitfall 6 (state separation — static world data loaded once, never written during play)

### Phase 3: Character and Combat
**Rationale:** Character creation depends on the account system (Phase 1). Combat depends on the room system (Phase 2). The D&D rules subset must be documented before combat code is written — this is a design gate, not an implementation step.
**Delivers:** Character creation (race, class, ability scores), HP/mana/stamina tracking, basic combat (attack roll vs AC, damage rolls, flee), NPC monsters with spawn tables and loot, inventory + body-slot equipment, death + bind-point respawn, player-to-player inspection.
**Addresses:** Character creation (P1), basic combat (P1), NPC monsters (P1), inventory/equipment (P1), player inspection (P1)
**Avoids:** Pitfall 9 (D&D rules scope creep — rules subset document is a prerequisite for this phase)

### Phase 4: TUI Client
**Rationale:** The Ratatui TUI is the product's primary visual differentiator. Building it after the server game loop is working means the UI can be tested against real game state from the start. Unicode width constraints must be established as hard constraints before any UI components are designed.
**Delivers:** Ratatui split-panel layout (room description pane, vitals bar, chat pane, input line), native terminal client binary (iTerm2 + xterm compatible), compass/minimap overlay, D&D dice roll display in combat log.
**Addresses:** Ratatui TUI layout (P1), native client (P1), minimap overlay (P2), dice roll display (P2)
**Avoids:** Pitfall 5 (Unicode width mismatch — restrict to U+2500-U+257F box-drawing; test on xterm explicitly)

### Phase 5: Chat and Social
**Rationale:** Chat is a social/exploration-first game's core engagement loop. Local and global channels, IC/OOC separation, and the channel scope architecture (room-scoped mpsc + global broadcast) must be built together to avoid the single-broadcast pitfall. Character biography and social polish layer in here.
**Delivers:** Local chat (say, emote, whisper), global gossip channel (OOC), newbie/help channel, IC/OOC channel separation, character biography field.
**Addresses:** Local chat (P1), global chat (P1), channel system (differentiator), character biography (P2)
**Avoids:** Pitfall 7 (single broadcast for all events — scoped channels implemented from the start, not retrofitted)

### Phase 6: Browser Client
**Rationale:** The xterm.js browser client removes the install barrier entirely. It depends on Phase 4's TUI server-side ANSI output and Phase 5's chat channels being stable. xterm.js-specific pitfalls (flow control, reconnect) are contained to this phase.
**Delivers:** xterm.js web client served as static assets, WebSocket connection to Axum server, parity with native client features, session reconnect with grace period (60s), watermark-based flow control.
**Addresses:** Browser client (P1 — barrier removal is a launch requirement)
**Avoids:** Pitfall 4 (xterm.js flow control), Pitfall 11 (WebSocket session reconnect), Pitfall 10 (protocol drift — same protocol as native client)

### Phase 7: Procedural Dungeons
**Rationale:** Procedural generation requires a stable room system (Phase 2) and a stable persistence model (Phase 1). The dungeon generator is a separate crate with no async dependencies — it can be developed in isolation and wired in here. Quality controls (BSP layout, loop-back edges, dungeon grammar, boss-room reachability) must be baked in from the start.
**Delivers:** dungeon-gen crate (BSP + cellular automata), room connectivity with loop-back edges (not just MST), dungeon grammar (entrance / middle / boss zone), hand-crafted set piece injection at anchor points, seed-based reproducibility, generated floors persisted in PostgreSQL, dungeon reset timers.
**Addresses:** Procedural dungeons (P2), hand-crafted set pieces (P2)
**Avoids:** Pitfall 8 (dungeon quality — BSP + grammar required, not optional), Pitfall 4 (generation on tick thread — spawn_blocking required)

### Phase 8: Web Armory
**Rationale:** The armory reads game data but does not affect it — it is architecturally independent after the character schema is stable. Deferred to after the game loop is proven and the character data model is not changing frequently. The armory is the only phase that involves the SvelteKit + Threlte stack.
**Delivers:** Axum REST API routes for armory (/characters, /armory, /auth), SvelteKit armory SPA, public character profiles (stats, equipped gear, level, class), 3D gear visualization via blend-ai-generated GLB models in Threlte, JWT auth for profile ownership claims.
**Addresses:** Web armory (P2), 3D gear visualization (P2)
**Avoids:** Pitfall (armory in game engine — reads PostgreSQL directly, zero game tick involvement)

### Phase Ordering Rationale

- Phases 1-3 are non-negotiable in this order: the three highest-recovery-cost pitfalls are all Phase 1 concerns. Skipping or deferring ECS design or the protocol crate contaminates all downstream phases.
- The feature dependency graph from FEATURES.md confirms this ordering: account/login (Phase 1) → character creation (Phase 3) → web armory (Phase 8); room system (Phase 2) → combat (Phase 3) → procedural dungeons (Phase 7).
- The TUI client (Phase 4) and browser client (Phase 6) are separable from game logic phases and can be developed in parallel with Phases 3/5 by a second stream, but Phase 4 must precede Phase 6.
- Web armory (Phase 8) is explicitly last because it requires stable character schema, stable Axum API, and the SvelteKit stack — none of which should be introduced until core game systems are proven.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 7 (Procedural Dungeons):** Dungeon grammar design and set piece injection patterns are lightly documented for Rust; recommend a research spike on BSP implementation specifics and the noise-rs API before planning tasks.
- **Phase 8 (Web Armory):** blend-ai 3D model generation API is not documented in this research; integration pattern (server-side generation vs client-side call), output format (GLB/GLTF), caching strategy, and Threlte model loading all need a dedicated research spike.
- **Phase 6 (Browser Client):** xterm.js flow control watermark implementation details and the specific Axum WebSocket + ANSI output pipeline are technically validated but not prototyped; recommend a proof-of-concept spike early in planning.

Phases with standard patterns (skip research-phase):
- **Phase 1 (Server Foundation):** Actor pattern, ECS with hecs, SQLx migrations — all well-documented with official examples. Alice Ryhl's "Actors with Tokio" is a canonical reference.
- **Phase 2 (World and Movement):** Room data model and movement dispatch are genre bedrock; no novel patterns required.
- **Phase 3 (Character and Combat):** D&D-flavored subset is intentionally simple; the design document (rules subset) is the work, not implementation research.
- **Phase 4 (TUI Client):** Ratatui 0.30 has extensive documentation and examples; split-panel layout is a known pattern.
- **Phase 5 (Chat):** tokio broadcast/mpsc scoped channel pattern is standard; no novel research required.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Core Rust ecosystem (Tokio, Axum, SQLx, Ratatui, Crossterm) verified against official release notes and crates.io. All versions confirmed current as of 2026-03-23. Threlte 8 + Svelte 5 requirement is MEDIUM — verify before web scaffolding. |
| Features | MEDIUM | MUD genre is well-documented; competitor analysis (Alter Aeon, Untold Dawn) is current. Modern MUD comparables are sparse, reducing confidence on what "modern players expect." WoW Armory as armory reference is HIGH confidence. |
| Architecture | MEDIUM-HIGH | Actor pattern and ECS architecture are validated by Rust community consensus (Alice Ryhl, tokio-rs community). No post-2023 production Rust MUD exists as a direct comparison point; architecture is synthesized from game server patterns, not MUD-specific prior art. |
| Pitfalls | MEDIUM | Tokio-specific pitfalls (blocking, lock contention) are HIGH confidence from official Tokio documentation. xterm.js flow control is confirmed via official xterm.js docs. MUD-specific design pitfalls (D&D scope creep, dungeon quality) are inferred from genre history and game design literature, not Rust MUD post-mortems. |

**Overall confidence:** MEDIUM-HIGH

### Gaps to Address

- **blend-ai integration:** No research was conducted on the blend-ai 3D model generation API. Before Phase 8 planning, determine: API authentication model, request/response format, GLB/GLTF output confirmation, rate limits, and whether model generation is synchronous or async with a webhook. This affects armory architecture and cost model.
- **hecs vs alternative ECS:** Research recommended hecs as "lightweight, no scheduler." Confirm hecs is actively maintained and that its API is sufficient for MUT's entity relationship model (rooms, players, monsters, items) before Phase 1 implementation. Alternative: `shipyard` or rolling a simple slotmap-based arena.
- **Threlte 8 + Svelte 5 compatibility:** STACK.md flags this as MEDIUM confidence. Scaffold a minimal Threlte 8 + SvelteKit 2 + Three.js r170 project and confirm GLB model loading before committing Phase 8 to this stack.
- **Connection volume target:** Architecture scaling notes suggest 0-500 concurrent players is single-server, no Redis needed. Clarify the actual target concurrent player count for v1 — this determines whether Redis is required in early phases or can be deferred entirely.
- **Dungeon grammar design:** The pitfalls and architecture research recommend a "dungeon grammar" (entrance / middle / boss zones) but do not specify parameters. This is a design decision (not a research gap) that should be resolved in Phase 7 planning before any generator code is written.

## Sources

### Primary (HIGH confidence)
- https://github.com/ratatui/ratatui/releases — Ratatui 0.30.0 confirmed Dec 2024
- https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0 — Axum 0.8 release notes
- https://docs.rs/crate/sqlx/latest — SQLx 0.8.6 confirmed
- https://github.com/xtermjs/xterm.js/releases — xterm.js 6.0.0 Dec 2024
- https://www.npmjs.com/package/@xterm/addon-attach — @xterm/addon-attach 0.12.0
- https://crates.io/crates/crossterm — crossterm 0.29.0
- https://github.com/sveltejs/kit/releases — SvelteKit 2.55.0 current stable
- https://ryhl.io/blog/actors-with-tokio/ — Canonical Rust actor pattern
- https://xtermjs.org/docs/guides/flowcontrol/ — xterm.js flow control guide
- https://tokio.rs/blog/2020-04-preemption — Tokio cooperative scheduling

### Secondary (MEDIUM confidence)
- https://inviocean.com/play/accessibility-as-a-matter-of-survival-mud-games-in-2025/ — Modern MUD design expectations
- https://writing-games.org/alter-aeon-mud/ — Alter Aeon feature analysis
- https://blog.untold-dawn.com/ — Modern Rust-based MUD design philosophy
- https://www.andrewzigler.com/blog/mud-cookbook-design-meets-implementation/ — MUD design pitfalls
- https://heroiclabs.com/docs/nakama/concepts/multiplayer/authoritative/ — Tick-based authoritative server architecture
- https://www.gamedeveloper.com/programming/procedural-dungeon-generation-algorithm — Dungeon generation patterns
- https://qouteall.fun/qouteall-blog/2025/How%20to%20Avoid%20Fighting%20Rust%20Borrow%20Checker — ECS/ownership patterns

### Tertiary (LOW confidence)
- https://github.com/duysqubix/MuOxi — Rust MUD reference (last active 2020; architecture patterns referenced, not code)
- https://threlte.xyz — Threlte 8.x Svelte 5 requirement (verify before scaffolding)
- https://github.com/Razaekel/noise-rs — noise-rs procedural generation (verify current version)

---
*Research completed: 2026-03-23*
*Ready for roadmap: yes*
