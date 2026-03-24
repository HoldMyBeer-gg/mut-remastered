# Pitfalls Research

**Domain:** Multi-User Text Dungeon (MUD) — Rust game server, Unicode TUI, dual-client (native + web), D&D mechanics, procedural generation
**Researched:** 2026-03-23
**Confidence:** MEDIUM (WebSearch + official docs; no authoritative MUD-in-Rust post-mortems exist)

---

## Critical Pitfalls

### Pitfall 1: Blocking Code on the Tokio Worker Thread

**What goes wrong:**
Synchronous I/O, heavy CPU computation (dungeon generation, pathfinding, dice simulation loops), or even a long-running mutex lock placed directly inside an `async` task stalls Tokio's worker threads. Because Tokio uses cooperative scheduling, one blocked worker can starve every other task on that thread. Under load this manifests as mysterious latency spikes, degraded tick rate, and unpredictable player experience — not a crash, so it is hard to detect until production.

**Why it happens:**
Developers treat `async fn` as "automatically non-blocking." It is not. Any synchronous call inside an async context blocks the thread. Procedural dungeon generation and pathfinding are classic CPU-bound operations that look innocent but routinely exceed the 100-microsecond budget that Tokio assumes for fair task scheduling.

**How to avoid:**
- Use `tokio::task::spawn_blocking` for CPU-bound work (dungeon gen, monster AI, complex dice rolls with many iterations).
- Wrap any synchronous third-party library call in `spawn_blocking`.
- Keep the game tick handler itself async and non-blocking; offload simulation sub-tasks.
- Install `tokio-blocked` (crate) in development to detect blocking regressions automatically.

**Warning signs:**
- Tick rate becomes inconsistent under multiple simultaneous connections.
- Player actions have variable latency even when the server CPU is not maxed.
- Tokio metrics show worker threads at 100% with low task throughput.

**Phase to address:** Core server infrastructure phase (game loop and networking foundation).

---

### Pitfall 2: Treating the Game Loop as a Single Async Task

**What goes wrong:**
Putting the entire game simulation — tick processing, entity updates, I/O dispatch, dungeon events — into one big async loop creates a serialization bottleneck. Every connected player waits behind every other player's processing in the same loop iteration. This works fine with 5 players and fails noticeably with 50.

**Why it happens:**
It is the simplest thing to write. One `loop { simulate(); broadcast(); sleep(tick_ms).await; }` works in a prototype. Refactoring it later requires restructuring nearly all game logic.

**How to avoid:**
- Design the game loop as three separate concerns from day one: (1) collect player commands, (2) simulate world state, (3) broadcast updates.
- Player command ingestion runs per-connection (one task per TCP/WebSocket connection).
- World simulation runs on a fixed tick (e.g., 200ms for a MUD — 5 Hz is fine for turn-paced play).
- Update broadcasting is a separate fan-out step.
- Use channels (tokio `mpsc` / `broadcast`) to decouple these stages.

**Warning signs:**
- Adding a single expensive NPC behavior slows all player input response times.
- Tick jitter increases proportionally with connected player count.

**Phase to address:** Core server infrastructure phase, before any gameplay is added.

---

### Pitfall 3: Borrow Checker Fighting with Shared World State

**What goes wrong:**
The most natural way to model a MUD world — a `World` struct holding rooms, entities, and players all borrowing each other — is rejected by the borrow checker. Attempts to fix this with `Rc<RefCell<>>` or `Arc<Mutex<>>` everywhere result in deadlocks, runtime panics, or lock contention that kills concurrency.

**Why it happens:**
Game world graphs are inherently self-referential: rooms reference monsters, monsters reference rooms, players hold references to their current room. Rust's ownership model does not permit circular borrows. Developers unfamiliar with ECS or arena patterns reach for smart pointers, which moves the problem to runtime.

**How to avoid:**
- Use an ECS or entity-handle pattern from the start. `hecs` (lightweight, no system scheduler) is the correct choice for a MUD where the "system" is the game tick handler, not a framework.
- Represent all entity relationships as IDs/indices into slotmap arenas, never as direct references.
- Use the deferred-mutation (command queue) pattern: during a tick, collect all mutations as commands; apply them after the read phase ends.
- Never hold a mutable borrow across an await point.

**Warning signs:**
- More than two levels of `Arc<Mutex<>>` nesting anywhere in the world model.
- `RwLock` held across async boundaries.
- Deadlock reproduced only under concurrent load.

**Phase to address:** Core server infrastructure phase — the ECS/world model must be established before any gameplay systems are built on top of it.

---

### Pitfall 4: xterm.js Flow Control Ignored

**What goes wrong:**
The server streams ANSI-escaped TUI frames to xterm.js via WebSocket. Under fast output (scrolling room descriptions, combat logs, area discovery) xterm.js's input buffer fills faster than its 5–35 MB/s rendering throughput can drain it. Data beyond the 50 MB hard-coded write buffer is silently discarded. The player sees truncated output with no error.

**Why it happens:**
Developers treat the WebSocket as a reliable lossless pipe with unlimited capacity. xterm.js documentation on flow control is not prominent, and the symptoms look like rendering bugs rather than buffer overflows.

**How to avoid:**
- Implement watermark-based flow control: track buffered bytes; pause sending at the HIGH watermark (~500 KB); resume at LOW watermark.
- Attach write callbacks (ACK messages) every ~100 KB, not per-write.
- Server must coordinate flow control at the application layer — WebSocket framing does not expose backpressure.
- Keep individual frame payloads small; prefer incremental diff updates over full redraws.

**Warning signs:**
- Web client shows partial room descriptions on fast area transitions.
- Native TUI client renders correctly while web client is garbled.
- xterm.js write buffer grows during high-output scenarios (inspect via xterm.js `_core.buffer`).

**Phase to address:** Web client integration phase.

---

### Pitfall 5: Unicode Character Width Mismatch Between Terminals

**What goes wrong:**
Box-drawing characters, emoji, and "ambiguous width" Unicode characters (CJK-adjacent range) render at different widths in iTerm2 vs xterm. A UI drawn to a cell grid on iTerm2 is misaligned on xterm because the same codepoint is treated as 1-wide by one terminal and 2-wide by the other. Ratatui uses `unicode-width` for layout — if the terminal disagrees with the library's table, every border and widget overlaps.

**Why it happens:**
Unicode's "East Asian Width" specification leaves a large set of characters as "ambiguous." Terminal implementations pick different Unicode version tables. Each Unicode version adds more full-width and zero-width characters, and terminals rarely update in lockstep.

**How to avoid:**
- Restrict the UI character palette to code points with unambiguous width (standard box-drawing: U+2500–U+257F, basic braille U+2800–U+28FF).
- Never use emoji as UI furniture — only as explicit content that can degrade gracefully.
- Test the exact character set on xterm (Linux) and iTerm2 (macOS) in CI with a screenshot diff.
- Use Ratatui's `SymbolSet::Unicode` mode but verify every custom symbol against `unicode-width` crate results.

**Warning signs:**
- Borders look correct in iTerm2 but have gaps or overlaps in xterm.
- Widget alignment shifts after adding a new decorative character.

**Phase to address:** TUI foundation phase, established as a constraint before any UI components are designed.

---

### Pitfall 6: Mixing Persistent World State with In-Memory Game State

**What goes wrong:**
World state (room definitions, loot tables, monster templates) and session state (player positions, current HP, active combat) get stored in the same database tables with the same flush cadence. Either: (a) session state is over-persisted (expensive DB writes every tick), or (b) world state is under-persisted (lost on crash). Crash recovery becomes non-deterministic.

**Why it happens:**
It is simpler to have one state model. The distinction between "what exists in the world" and "what is happening right now" is clear conceptually but blurs during implementation.

**How to avoid:**
- Separate state into three layers: (1) static world data (rooms, templates) — load once at startup, never write during play; (2) persistent character state (XP, inventory, level) — write on meaningful events (level up, zone change, logout); (3) ephemeral session state (position, current HP, combat round) — in-memory only, reconstructed on reconnect.
- Use SQLx with explicit query control rather than an ORM; ORMs hide which queries are executing and their frequency.
- Write session state to DB only on disconnect, zone transition, or explicit save commands.

**Warning signs:**
- DB query count scales linearly with connected player count per tick.
- Reconnect after crash loses position but not inventory (signals mixed flush strategies).

**Phase to address:** Persistence layer phase, before player sessions are implemented.

---

### Pitfall 7: Single WebSocket Broadcast for All Chat and Game Events

**What goes wrong:**
A single broadcast channel sends all events (global chat, local chat, combat log, ambient descriptions) to all connected players. Players in Room A receive Room B's combat events and filter client-side. As player count grows, bandwidth explodes at O(n) per event regardless of relevance. With 100 players this is tolerable; with 500 it saturates connections and causes in-game lag.

**Why it happens:**
`tokio::sync::broadcast` is a one-liner. It works perfectly during solo testing and early multiplayer testing. The architectural flaw only appears at scale.

**How to avoid:**
- Model event channels as scoped from the start: per-room channel, per-zone channel, server-wide channel.
- Each player subscribes only to relevant channels (their room + global).
- Implement chat scope server-side, not client-side filtering.
- Use `tokio::sync::broadcast` for global events only; use `mpsc` per-room or per-zone for local events.

**Warning signs:**
- Player connection bandwidth is proportional to total server player count, not local player density.
- Global chat causes CPU spike during busy hours.

**Phase to address:** Chat and event system phase.

---

### Pitfall 8: Procedural Dungeon Generation Without Quality Controls

**What goes wrong:**
Pure random dungeon generation produces dungeons that are either (a) impossible to complete (critical loot/boss unreachable), (b) trivially easy (all rooms accessible from spawn), or (c) structurally homogeneous (all dungeons feel identical after a few runs). A minimum spanning tree approach — the simplest way to guarantee connectivity — creates linear dungeons with a single critical path and no interesting loops or shortcuts.

**Why it happens:**
Connectivity is the first property developers test. Once "you can reach every room" is confirmed, variety feels solved. The narrative and difficulty problems only appear during sustained play.

**How to avoid:**
- Use BSP (Binary Space Partitioning) or cellular automata for initial room layout, not pure random placement.
- After MST connectivity pass, add back 15–20% of removed edges to create loops.
- Define a "dungeon grammar": entrance zone (low danger), middle (escalating), boss zone (high danger, guaranteed reward). Generate within this grammar, not fully open.
- Hand-authored set pieces and boss rooms are a design constraint, not optional polish — their fixed placement anchors generated content around them.
- Seed RNG per dungeon and store the seed; reproducibility enables debugging and player sharing.

**Warning signs:**
- Two adjacent dungeons feel structurally identical.
- Players report dead-end dungeons where the boss room is inaccessible.
- All critical items always appear in predictable room positions.

**Phase to address:** Procedural generation phase. Set piece system should be scaffolded before random generation is layered on top.

---

### Pitfall 9: D&D Rules Ported Literally Instead of Adapted

**What goes wrong:**
Implementing full D&D 5e rules (action economy, reaction timing, concentration spells, opportunity attacks, exhaustion levels, full condition list) creates an enormous implementation surface. Bugs in rules interactions are hard to find and fix. More critically, rules designed for a human DM mediating social interaction feel mechanical and punishing when enforced rigidly by code. The game becomes a rules lawyering simulator rather than a social exploration experience.

**Why it happens:**
D&D is a well-documented reference system. Using it verbatim avoids design decisions. The cost is invisible until the implementation surface is already large.

**How to avoid:**
- Define the MUT Remastered subset explicitly at the start: ability scores (6 scores, modifiers), armor class, hit points, attack roll, saving throw, skill check. That is the core.
- Explicitly cut: concentration, opportunity attacks, action/bonus action/reaction distinctions, condition interactions beyond incapacitated/dead.
- Represent combat as: attack roll vs AC = hit/miss; damage roll; HP depletion. Nothing more for v1.
- Write the rule subset as a document (not code) first; review for simplicity before implementing.

**Warning signs:**
- Combat code references more than 10 condition states.
- Any rule requires "checking if another rule applies first."
- A combat turn requires more than 3 decisions from the player.

**Phase to address:** Character and combat system phase. Rules must be scoped in design before any combat code is written.

---

### Pitfall 10: Dual-Client Protocol Drift

**What goes wrong:**
The native Ratatui client and the xterm.js web client start sharing protocol but gradually diverge: the native client gets features first (because it is easier to test), web client falls behind, and eventually the server develops implicit assumptions about which client sent which message. The "same experience" guarantee erodes invisibly.

**Why it happens:**
Native client development is faster. Each "temporary" shortcut for the web client accumulates. There is no enforced contract between clients and server.

**How to avoid:**
- Define a single versioned protocol (message types, field schemas) as the canonical interface before building either client.
- Both clients implement the same protocol; the server never needs to know which client type is connected.
- Use a shared type definition for protocol messages (e.g., a JSON schema or Rust types compiled to WASM for the web client, or a schema-first approach).
- Run integration tests against both clients from the same test suite.

**Warning signs:**
- A feature works in native client but is "not yet supported" in web client for more than one sprint.
- Server code has `if client_type == Native` branches.

**Phase to address:** Protocol definition phase, before either client is built beyond the skeleton.

---

### Pitfall 11: WebSocket Session Reconnect Not Handled

**What goes wrong:**
xterm.js WebSocket connections drop silently (network hiccup, browser tab backgrounded, mobile sleep). If the server treats connection drop as logout, all in-progress state (current dungeon, active combat, party composition) is lost. Players who reconnect find themselves at the last explicit save point. This is a critical trust-destroying experience in a social/exploration game.

**Why it happens:**
TCP disconnects are obvious; WebSocket drops are not. The server's disconnect callback fires the same way for intentional logouts and network interruptions. Most tutorials treat disconnect as permanent.

**How to avoid:**
- Implement a session token that survives WebSocket disconnection. On reconnect, the client sends the token; server resumes the session.
- Keep session state in-memory for a configurable grace period (e.g., 60 seconds) before treating it as a permanent logout.
- Heartbeat ping/pong per RFC 6455 section 5.5 to detect dead connections before they go silent.
- Emit a "player reconnected" event to the room rather than "player left / player joined."

**Warning signs:**
- Playtests show players repeatedly logging back in from the hub rather than resuming.
- "Player left" messages fire during brief network interruptions.

**Phase to address:** Networking and session management phase.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| `Arc<Mutex<World>>` for all world state | Fast to write, works in prototype | Lock contention at scale; deadlock risk; makes async unsafe | Never — use ECS/handles from day one |
| Single broadcast channel for all events | One-liner with tokio | O(n) bandwidth waste; global chat causes system-wide CPU spike | Never for global broadcast of local events |
| ORM for game state persistence | Quick schema setup | Hidden query patterns; N+1 queries on entity load; hard to optimize | Never for hot-path game state; acceptable for user account management only |
| Full D&D 5e rules | No design decisions needed | Massive implementation surface; rules-lawyering gameplay | Never — always define a subset |
| Game loop as single async task | Simple to write | Serialization bottleneck; tick jitter under load | Prototype only; must refactor before adding second player |
| Hard-coded room connections | Easy for first dungeon | Cannot support procedural generation | Prototype/set pieces only — use data-driven room model from start |
| xterm.js without flow control | Works in local testing | Silent data loss on fast output in production | Never — implement watermarks before first web client demo |
| Storing RNG state globally | Simpler code | Non-reproducible dungeons; impossible to debug generation bugs | Never — seed per dungeon, store seed |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| xterm.js + WebSocket | Treating WebSocket as unlimited pipe; no flow control | Watermark-based ACK flow control; max ~500 KB HIGH watermark |
| xterm.js + WebSocket | No heartbeat; session freezes silently | RFC 6455 ping/pong heartbeat every 30s; reconnect on missed pong |
| Ratatui + xterm terminal | Using ambiguous-width Unicode for UI chrome | Restrict to unambiguous box-drawing U+2500–U+257F; test on xterm explicitly |
| SQLx + game tick | Writing to DB on every tick | Write only on events (login, logout, level up, zone change) |
| tokio + CPU-bound generation | Dungeon gen blocking worker thread | `tokio::task::spawn_blocking` for all gen tasks |
| hecs ECS + async | Holding ECS world borrow across `.await` | Command queue pattern: collect mutations, release borrow, apply commands |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Full world state broadcast per tick | Bandwidth grows with player count regardless of activity | Scoped event channels (room/zone/global) | ~20 concurrent players on a busy server |
| Dungeon generation on the game tick thread | Tick jitter spikes when a new dungeon generates | `spawn_blocking` for generation; pre-generate dungeons async | First dungeon generation during active play |
| Linear MST dungeon connectivity check | All dungeons feel identical; O(n²) room graph on large dungeons | BSP layout + loop-back edges; cap dungeon size | Dungeons > 50 rooms |
| Naive room description reload on every player enter | DB reads proportional to movement rate | Cache static room data in-memory at startup; never re-read static world data | > 10 room transitions per second |
| Shared `Mutex<World>` accessed per tick | Lock contention causes tick jitter | ECS with per-system borrows; no global world lock | > 3 concurrent player commands in same tick |
| Broadcasting to all connected clients for local chat | CPU + bandwidth scales with total player count | Room-scoped channels; subscribe per room on enter, unsubscribe on exit | > 50 players with active chat |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Trusting client-reported position/stats | Teleport exploits, stat inflation | Server is authoritative; client sends only intent (move direction, action type), never position or computed values |
| Unbounded chat message length | Memory exhaustion, DB bloat | Enforce max message length server-side (e.g., 500 chars); truncate, not error |
| Session tokens transmitted in URL | Token exposure in logs, referrer headers | Session tokens in WebSocket protocol headers or first-message auth, never URL params |
| Deterministic RNG seeded from player ID | Players can predict loot/dungeon layout | Seed dungeon RNG from server-side cryptographic random; do not expose seed to client |
| No rate limiting on player commands | Automated bots, server spam | Per-connection command rate limit (e.g., 20 commands/second) before game logic processes them |
| Account enumeration via login errors | Username harvesting | Return identical error message for "wrong password" and "user not found" |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Lost session on brief disconnect | Players restart from last save; discourages mobile/web use | Session grace period (60s); reconnect resumes exactly where player was |
| No feedback during dungeon load | Player sees blank screen; thinks client crashed | "Generating dungeon..." progress indicator or animated waiting state in TUI |
| Dungeon too linear (MST-only connectivity) | Game feels like a corridor; no discovery | Add loop-back edges; ensure at least 2 paths between key nodes |
| Room description wall of text on every enter | Players skip descriptions after second visit | Show brief description on re-enter; full description only on first visit or explicit `look` command |
| Combat log mixed with chat | Social interaction buried in combat noise | Separate scrollable panels for combat log vs chat; Ratatui supports split views |
| Web client falls behind native client features | Web players feel like second-class; discourages browser use | Feature parity gate: no feature ships to native that is not also in web client |
| Character stats panel requires menu navigation | Players cannot check stats mid-combat | Always-visible stats sidebar in TUI layout; character panel toggled without leaving game view |

---

## "Looks Done But Isn't" Checklist

- [ ] **Game loop:** Tick runs at fixed interval under load — verify with 20 simulated clients, not 1.
- [ ] **xterm.js client:** Web client tested with fast text output (large dungeon room dump) — verify no truncation.
- [ ] **TUI layout:** All widgets tested in xterm (Linux) at 80x24, 120x40, and 200x50 column widths — not just iTerm2.
- [ ] **Reconnect:** Dropped WebSocket mid-dungeon results in resume, not logout — verify with network partition test.
- [ ] **Procedural dungeon:** Every generated dungeon has a reachable boss room — verify with automated pathfinding check on 1000 generated samples.
- [ ] **D&D combat:** Dice rolls are server-side and deterministic per seed — verify client cannot influence roll outcome.
- [ ] **Chat scoping:** Local chat message from Room A does not appear in Room B — verify with two simultaneous connections in different rooms.
- [ ] **Persistence:** Character state survives server restart — verify by killing the server process mid-session and reconnecting.
- [ ] **Unicode width:** Box-drawing characters align correctly in both iTerm2 and a plain xterm instance — verify visually before declaring TUI "done."
- [ ] **ECS world state:** No direct references between entities — verify no `Arc<Mutex<Entity>>` patterns in world model.

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Blocking code on Tokio workers | MEDIUM | Profile with `tokio-console`; identify blocking tasks; wrap with `spawn_blocking`; no architectural rewrite needed |
| Monolithic game loop | HIGH | Requires decomposing into producer/consumer pipeline; all game logic must be audited for which stage it belongs to |
| `Arc<Mutex<World>>` everywhere | HIGH | Full ECS refactor; all gameplay code that touches world must be rewritten to use handles |
| xterm.js flow control missing | LOW | Add watermark ACK protocol between server and web client; localized change |
| Unicode misalignment in TUI | LOW–MEDIUM | Audit character set; replace offending codepoints with unambiguous alternatives |
| Protocol drift between clients | MEDIUM | Define canonical protocol schema; audit both clients against it; add integration tests |
| No session reconnect | MEDIUM | Add session token store and grace period; requires changes to connection lifecycle but not game logic |
| Dungeon generation quality | LOW | Tune generator constraints and grammar; no architectural impact |
| D&D rules scope creep | HIGH | Requires design decision to cut rules; all dependent combat code must be re-evaluated |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Blocking code on Tokio workers | Phase 1: Server foundation | Fixed-tick benchmark with `spawn_blocking` instrumented; no jitter under load |
| Monolithic game loop | Phase 1: Server foundation | Architecture review: three-stage pipeline present before gameplay code added |
| Borrow checker / ECS world model | Phase 1: Server foundation | No `Arc<Mutex<Entity>>` in world module; all relations are ID-based |
| Persistent vs session state separation | Phase 2: Persistence layer | DB write count per tick is O(1) regardless of player count |
| D&D rules subset scoped | Phase 3: Character/combat design | Rules document written and reviewed before any combat code is committed |
| TUI Unicode width constraints | Phase 4: TUI foundation | Automated screenshot diff between iTerm2 and xterm passes |
| xterm.js flow control | Phase 5: Web client integration | Stress test: 10 MB room dump to web client without truncation |
| Dual-client protocol drift | Phase 5: Web client integration | Both clients pass identical integration test suite |
| WebSocket reconnect | Phase 5: Web client integration | Network partition test: session resumes after 10s disconnect |
| Scoped event channels | Phase 6: Chat and events | Local chat isolation test: two rooms, zero cross-contamination |
| Procedural dungeon quality | Phase 7: Dungeon generation | Automated pathfinding check on 1000 samples: 100% boss-room reachability |
| Dungeon generation on tick thread | Phase 7: Dungeon generation | Tick jitter < 5ms during concurrent dungeon generation |

---

## Sources

- [Top 5 Tokio Runtime Mistakes That Quietly Kill Your Async Rust](https://www.techbuddies.io/2026/03/21/top-5-tokio-runtime-mistakes-that-quietly-kill-your-async-rust/)
- [Tokio: Reducing tail latencies with automatic cooperative task yielding](https://tokio.rs/blog/2020-04-preemption)
- [xterm.js Flow Control Guide](https://xtermjs.org/docs/guides/flowcontrol/)
- [xterm.js: Overcome network latency — Issue #887](https://github.com/xtermjs/xterm.js/issues/887)
- [How to Avoid Fighting the Rust Borrow Checker](https://qouteall.fun/qouteall-blog/2025/How%20to%20Avoid%20Fighting%20Rust%20Borrow%20Checker)
- [Rust Entity Component Systems overview](https://rodneylab.com/rust-entity-component-systems/)
- [Unicode East Asian Width UAX #11](http://www.unicode.org/reports/tr11/)
- [Terminal Wide Character Width Solution](https://www.jeffquast.com/post/terminal_wcwidth_solution/)
- [Procedural Generation: Golden Ticket or Gilded Cage](https://www.wayline.io/blog/procedural-generation-golden-ticket-or-gilded-cage)
- [Procedural Dungeon Generation Algorithm — Game Developer](https://www.gamedeveloper.com/programming/procedural-dungeon-generation-algorithm)
- [WebSocket Architecture Best Practices — Ably](https://ably.com/topic/websocket-architecture-best-practices)
- [Accurate tick rate for game servers without locking a thread](https://gamedev.net/forums/topic/713315-accurate-tick-rate-for-game-servers-without-locking-a-thread/)
- [Source Multiplayer Networking — Valve Developer Community](https://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking)
- [MUD Cookbook: design meets implementation](https://www.andrewzigler.com/blog/mud-cookbook-design-meets-implementation/)
- [Seven Design Mistakes Roleplaying Games Keep Making — Mythcreants](https://mythcreants.com/blog/seven-design-mistakes-roleplaying-games-keep-making/)
- [Rust ORMs in 2026: Diesel vs SQLx vs SeaORM — Medium](https://aarambhdevhub.medium.com/rust-orms-in-2026-diesel-vs-sqlx-vs-seaorm-vs-rusqlite-which-one-should-you-actually-use-706d0fe912f3)

---
*Pitfalls research for: MUD game server (Rust), Unicode TUI, dual-client, D&D mechanics, procedural generation*
*Researched: 2026-03-23*
