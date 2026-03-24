# Architecture Research

**Domain:** Multi-User Text Dungeon (MUD) game server with dual TUI clients
**Researched:** 2026-03-23
**Confidence:** MEDIUM-HIGH

---

## Standard Architecture

### System Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                          CLIENT LAYER                                 │
│                                                                        │
│  ┌─────────────────────┐     ┌──────────────────────────────────────┐ │
│  │  Native TUI Client  │     │         Web Frontend                 │ │
│  │  (Rust / Ratatui)   │     │  ┌──────────────┐  ┌─────────────┐  │ │
│  │                     │     │  │  xterm.js    │  │  Armory SPA │  │ │
│  │  TCP/WebSocket conn │     │  │  TUI Client  │  │  (React/SV) │  │ │
│  └────────┬────────────┘     │  └──────┬───────┘  └──────┬──────┘  │ │
│           │                  │         │                  │          │ │
└───────────┼──────────────────┼─────────┼──────────────────┼──────────┘
            │ Raw TCP (port 3000)        │ WebSocket (wss://)│ HTTP REST
            │                  │         │                  │
┌───────────┼──────────────────┼─────────┼──────────────────┼──────────┐
│           │        NETWORK GATEWAY     │                  │           │
│  ┌────────▼────────────────────────────▼─────┐  ┌────────▼────────┐  │
│  │          Protocol Handler                  │  │   HTTP/REST API │  │
│  │  - Raw TCP codec (Ratatui protocol)        │  │  (Axum)         │  │
│  │  - WebSocket framing (tokio-tungstenite)   │  │  - /characters  │  │
│  │  - Session auth + token validation         │  │  - /armory      │  │
│  │  - Input/output serialization (JSON/ANSI)  │  │  - /auth        │  │
│  └─────────────────────┬──────────────────────┘  └─────────────────┘  │
│                        │ mpsc channels                                 │
└────────────────────────┼──────────────────────────────────────────────┘
                         │
┌────────────────────────┼──────────────────────────────────────────────┐
│                    GAME ENGINE CORE                                     │
│                                                                         │
│  ┌──────────────────────▼──────────────────────────────────────────┐   │
│  │                    World Actor (tick loop)                        │   │
│  │   - Fixed tick rate (e.g., 20 ticks/sec)                         │   │
│  │   - Schedules system execution order                              │   │
│  │   - Routes commands to Room Actors / ECS systems                 │   │
│  └───┬───────────────┬────────────────┬────────────────┬────────────┘   │
│      │               │                │                │                 │
│  ┌───▼───┐       ┌───▼───┐       ┌───▼────┐      ┌───▼─────┐           │
│  │ Room  │       │ Room  │  ...  │ Combat │      │  Chat   │           │
│  │ Actor │       │ Actor │       │ System │      │ System  │           │
│  │       │       │       │       │        │      │         │           │
│  │Manages│       │Manages│       │D&D     │      │Local +  │           │
│  │local  │       │players│       │rolls,  │      │Global   │           │
│  │state  │       │in room│       │HP, AC  │      │channels │           │
│  └───────┘       └───────┘       └────────┘      └─────────┘           │
│                                                                          │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                   ECS World (hecs / legion)                        │  │
│  │   Entities: players, monsters, items, rooms                        │  │
│  │   Components: Position, Health, Stats, Inventory, Description...   │  │
│  │   Systems: MovementSystem, CombatSystem, SpawnSystem, DecaySystem  │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                                                                          │
└────────────────────────┬─────────────────────────────────────────────────┘
                         │
┌────────────────────────┼──────────────────────────────────────────────┐
│                    DUNGEON GENERATION                                   │
│                                                                         │
│  ┌──────────────────────▼────────────────────────────────────────────┐  │
│  │                Generation Pipeline                                  │  │
│  │                                                                     │  │
│  │  BSP/Cellular → Graph Layout → Room Placement → Connector Pass      │  │
│  │       ↓               ↓              ↓                ↓            │  │
│  │  Raw grid        Room graph      Hand-crafted     Finalized        │  │
│  │  carving         adjacency       set piece         dungeon         │  │
│  │                                  injection         data            │  │
│  └───────────────────────────────────────────────────────────────────┘  │
└────────────────────────┬──────────────────────────────────────────────┘
                         │
┌────────────────────────┼──────────────────────────────────────────────┐
│                    PERSISTENCE LAYER                                    │
│                                                                         │
│  ┌──────────────────┐   ┌─────────────────┐   ┌───────────────────┐   │
│  │   PostgreSQL      │   │   Redis / Valkey │   │   File Store      │   │
│  │                  │   │                 │   │                   │   │
│  │  Characters      │   │  Active session │   │  Dungeon layouts  │   │
│  │  Accounts        │   │  state cache    │   │  Hand-crafted     │   │
│  │  World objects   │   │  Room presence  │   │  room templates   │   │
│  │  Dungeon seeds   │   │  Chat pub/sub   │   │  (TOML/RON)       │   │
│  │  Inventory       │   │  Combat state   │   │                   │   │
│  └──────────────────┘   └─────────────────┘   └───────────────────┘   │
└────────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Implementation |
|-----------|----------------|----------------|
| Native TUI Client | Rich Unicode ANSI rendering via Ratatui; reads stdin, sends commands over TCP/WS | Rust binary, separate crate |
| Web Frontend (xterm.js) | Browser TUI via ANSI over WebSocket; identical capabilities to native | TypeScript + xterm.js, served as static assets |
| Armory SPA | Character profiles, public gear visualization, 3D model display, account hub | React or SvelteKit, communicates via REST API |
| Protocol Handler | Normalizes raw TCP + WebSocket connections into a uniform `PlayerInput` stream | Rust, tokio-tungstenite + custom TCP codec |
| HTTP/REST API | Serves armory data, auth tokens, account management | Axum |
| World Actor | Owns the authoritative tick loop; routes player commands to appropriate systems | Rust task with tokio::interval |
| Room Actors | Each room owns its player list, item list, local events | Rust tasks communicating via mpsc |
| ECS World | All live game entities and their components; queried by systems each tick | hecs (lightweight) or legion |
| Combat System | D&D-flavored dice rolls, HP/AC resolution, death handling | ECS system running per tick |
| Chat System | Local room broadcast + global channel pub/sub | In-process for local; Redis pub/sub for global |
| Dungeon Generator | Offline + on-demand procedural generation; injects hand-crafted set pieces | Standalone Rust crate |
| PostgreSQL | Authoritative durable state: characters, accounts, world objects, dungeon seeds | sqlx async driver |
| Redis / Valkey | Ephemeral hot state: sessions, room occupancy, chat routing, combat locks | fred or redis-rs async |

---

## Recommended Project Structure

```
mut_remastered/
├── server/                   # Game server binary
│   ├── src/
│   │   ├── main.rs           # Startup: bind ports, init world, spawn tick loop
│   │   ├── network/          # Protocol handler layer
│   │   │   ├── tcp.rs        # Raw TCP codec + framing
│   │   │   ├── websocket.rs  # WebSocket upgrade + framing
│   │   │   └── session.rs    # Per-player session actor
│   │   ├── world/            # Core game engine
│   │   │   ├── mod.rs        # World actor, tick scheduler
│   │   │   ├── room.rs       # Room actor
│   │   │   ├── ecs.rs        # ECS world setup, component types
│   │   │   └── systems/      # ECS systems (combat, movement, spawn, decay)
│   │   ├── commands/         # Player command parsing and dispatch
│   │   ├── chat/             # Local + global channel logic
│   │   ├── auth/             # Account + token management
│   │   ├── db/               # sqlx queries, migrations
│   │   └── api/              # Axum REST routes for armory
│   └── Cargo.toml
│
├── client-native/            # Native TUI client binary
│   ├── src/
│   │   ├── main.rs           # Connect, event loop
│   │   ├── ui/               # Ratatui layout, widgets, input handling
│   │   └── protocol.rs       # Client-side message codec
│   └── Cargo.toml
│
├── dungeon-gen/              # Dungeon generation library (no server deps)
│   ├── src/
│   │   ├── lib.rs
│   │   ├── bsp.rs            # Binary space partition algorithm
│   │   ├── cellular.rs       # Cellular automata for caves
│   │   ├── graph.rs          # Room connectivity graph
│   │   ├── setpiece.rs       # Hand-crafted room injection
│   │   └── templates/        # RON/TOML files for set pieces
│   └── Cargo.toml
│
├── web/                      # Web frontend (xterm.js + armory)
│   ├── src/
│   │   ├── terminal/         # xterm.js TUI client
│   │   └── armory/           # Character armory SPA
│   ├── package.json
│   └── vite.config.ts
│
├── shared/                   # Protocol types shared by server + clients
│   ├── src/
│   │   └── protocol.rs       # Enums: ClientMessage, ServerMessage
│   └── Cargo.toml
│
└── Cargo.toml                # Workspace
```

### Structure Rationale

- **dungeon-gen/ as separate crate:** No dependencies on server runtime; can be tested independently and run offline as a CLI tool for map design.
- **shared/ protocol crate:** Both native client and server compile against the same message types, preventing codec drift.
- **network/ layer isolation:** The rest of the server never sees raw bytes; it receives typed `PlayerInput` events and sends typed `ServerMessage` values.
- **web/ as a separate build:** Vite bundles the frontend; the server serves the static output, keeping frontend dev tools out of Rust build.

---

## Architectural Patterns

### Pattern 1: Actor-per-Session with Shared World

**What:** Each connected player is a Tokio task (session actor) that owns its I/O. A central World actor owns authoritative state and processes commands in a tick loop. Session actors send commands to the World actor; World actor broadcasts events back to session actors.

**When to use:** Always for this project. Text MUDs have low tick requirements (1-20 Hz) and high concurrency (hundreds of idle connections). Tokio tasks are near-free for idle connections.

**Trade-offs:** Simple and idiomatic in Rust. No shared mutable state — only message passing. Bounded `mpsc` channels provide natural backpressure. Slightly more boilerplate than a shared mutex approach, but eliminates data races entirely.

**Example:**
```rust
// Session actor: owns the I/O, sends commands upward
async fn run_session(
    conn: WebSocketStream<TcpStream>,
    world_tx: mpsc::Sender<WorldCommand>,
    mut session_rx: mpsc::Receiver<ServerMessage>,
) {
    loop {
        tokio::select! {
            Some(msg) = conn.next() => {
                let cmd = parse_input(msg);
                world_tx.send(WorldCommand::PlayerInput { player_id, cmd }).await?;
            }
            Some(event) = session_rx.recv() => {
                let frame = serialize_event(event);
                conn.send(frame).await?;
            }
        }
    }
}
```

### Pattern 2: Fixed Tick Loop in World Actor

**What:** The World actor runs on a `tokio::interval` (e.g., every 50ms = 20 ticks/sec). Each tick: drain the command queue, run ECS systems in order, emit events to affected sessions.

**When to use:** All authoritative game simulation — combat resolution, monster AI, item decay, spawn timers. Not for pure I/O (session management remains event-driven).

**Trade-offs:** Predictable and debuggable. Easy to replay/rewind for testing. Tick rate determines responsiveness; 20 ticks/sec is ample for text input. Avoids the complexity of per-event locking.

**Example:**
```rust
async fn world_loop(mut rx: mpsc::Receiver<WorldCommand>, mut ecs: World) {
    let mut interval = tokio::time::interval(Duration::from_millis(50));
    loop {
        interval.tick().await;
        // Drain all queued commands
        while let Ok(cmd) = rx.try_recv() {
            apply_command(&mut ecs, cmd);
        }
        // Run ECS systems
        run_movement_system(&mut ecs);
        run_combat_system(&mut ecs);
        run_spawn_system(&mut ecs);
        // Emit outbound events
        flush_events(&ecs).await;
    }
}
```

### Pattern 3: Dual Transport with Shared Wire Protocol

**What:** Define a single `ClientMessage` / `ServerMessage` enum (in the shared crate, serialized as JSON or MessagePack). Both raw TCP (native client) and WebSocket (web client) carry the same messages. The protocol handler normalizes both transports before anything reaches the game engine.

**When to use:** Any project with two client types. This is the key to making native and web clients identical in capability.

**Trade-offs:** Slightly more overhead than a binary-only protocol, but JSON is debuggable and sufficient for text games. MessagePack is a drop-in if bandwidth becomes a concern. Avoids maintaining two separate codecs inside the game engine.

### Pattern 4: Dungeon Generation Pipeline (Offline + On-Demand)

**What:** Dungeons are generated offline (stored as seeds + room graphs in PostgreSQL), not synthesized per-player-entry. Hand-crafted set pieces are defined as RON/TOML templates and injected into the procedural graph at defined anchor points during generation.

**When to use:** Persistent worlds where room state (items, monsters, player traces) must survive reboots. Procedural generation happens when a dungeon floor is first "discovered" (or regenerated after a reset timer).

**Trade-offs:** Slightly less fresh than fully dynamic generation, but enables persistence. The generation crate stays pure and has no async dependencies, making it easy to test.

### Pattern 5: Chat via In-Process Broadcast + Optional Redis Pub/Sub

**What:** Local chat (same room) is a direct broadcast from the Room Actor to all session actors subscribed to that room — no external broker needed. Global chat uses a tokio `broadcast` channel (in-process for single-server deployments) that could be backed by Redis pub/sub if multi-server sharding is needed later.

**When to use:** This two-tier approach covers all v1 requirements while leaving a clean upgrade path.

**Trade-offs:** Simple and zero-latency for local chat. Global broadcast can degrade under very high message volumes, but this is not a realistic concern for v1.

---

## Data Flow

### Player Command Flow

```
Player types "go north"
    │
    ▼
Session Actor
    │  (parses raw bytes into ClientMessage::Command { text: "go north" })
    │
    ▼
World Actor command queue
    │  (drained each tick)
    │
    ▼
Command Dispatcher
    │  (matches "go north" → MovementCommand { direction: North })
    │
    ▼
MovementSystem (ECS tick)
    │  (validates destination room, checks locks/conditions)
    │  (updates Position component on player entity)
    │  (emits RoomEvent::PlayerEntered, RoomEvent::PlayerLeft)
    │
    ▼
Event Dispatcher
    │  (fans out ServerMessage::RoomDescription to player)
    │  (fans out ServerMessage::PlayerLeft to old room occupants)
    │  (fans out ServerMessage::PlayerEntered to new room occupants)
    │
    ▼
Session Actors receive messages → encode → send over transport
```

### Authentication + Session Flow

```
Client connects (TCP or WS)
    │
    ▼
Protocol Handler
    │  (reads ClientMessage::Login { username, password })
    │
    ▼
Auth Module → PostgreSQL
    │  (validates credentials, loads account, issues session token)
    │
    ▼
Session Actor created, player entity spawned in ECS
    │  (Position set to last known room from DB)
    │
    ▼
RoomActor for that room notified → sends RoomDescription to session
```

### Armory Data Flow

```
Browser requests /armory/characters/:id
    │
    ▼
Axum REST handler
    │  (no game engine involvement — reads PostgreSQL directly)
    │
    ▼
PostgreSQL: characters, inventory, stats tables
    │
    ▼
JSON response to browser
    │
    ▼
Armory SPA renders character + calls blend-ai 3D model endpoint
```

### Dungeon Generation Flow

```
Floor "discovered" for first time (or reset timer fires)
    │
    ▼
Dungeon Generator crate invoked
    │  (BSP split → room graph → corridor connector → set piece injector)
    │  (deterministic from seed)
    │
    ▼
Room definitions written to PostgreSQL
    │  (room_id, description, exits, monster_spawn_table, item_spawn_table)
    │
    ▼
World Actor loads rooms into ECS on-demand as players enter
```

### Chat Data Flow (Local)

```
Player sends "say Hello"
    │
    ▼
Session Actor → World Actor command queue
    │
    ▼
ChatSystem (tick)
    │  (resolves room occupants from Room Actor)
    │  (emits ServerMessage::Chat { channel: Local, text } to all occupants)
    │
    ▼
Room Actor broadcasts to session actors in room
```

### Chat Data Flow (Global)

```
Player sends "global Hello"
    │
    ▼
ChatSystem
    │  (publishes to tokio broadcast channel: GlobalChannel)
    │
    ▼
All connected session actors subscribed to GlobalChannel receive message
    │  (each session encodes and sends over its transport)
```

---

## Integration Points

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| Session Actor ↔ World Actor | `mpsc::Sender<WorldCommand>` (inbound), `mpsc::Sender<ServerMessage>` (outbound) | Bounded channels for backpressure |
| World Actor ↔ Room Actor | `mpsc::Sender<RoomEvent>` | Room actors are lightweight; one per loaded room |
| World Actor ↔ ECS | Direct function calls within same task | ECS is not shared across threads; owned by the tick loop |
| Game Engine ↔ PostgreSQL | `sqlx` async pool | Used at login, world load, and explicit save points — not every tick |
| Game Engine ↔ Redis | `fred` async client | Session presence, global chat, ephemeral combat state |
| Server ↔ Web Frontend | WebSocket (game) + HTTP (REST armory) | Two separate ports or path-based routing via Axum |
| Dungeon Gen ↔ Server | Function call (in-process) or CLI subprocess | Generator crate has no async; call synchronously in a `spawn_blocking` |

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| PostgreSQL | sqlx async pool, migrations via sqlx-cli | Source of truth for durable state |
| Redis / Valkey | fred crate, connection pool | Session cache, pub/sub for global chat; optional for single-server |
| blend-ai (3D models) | HTTP API call from Armory SPA or server-side at character creation | Async REST call; cache generated model URLs in DB |

---

## Scaling Considerations

| Scale | Architecture Adjustment |
|-------|--------------------------|
| 0-500 concurrent players | Single server binary, in-process broadcast for all channels, no Redis needed |
| 500-2000 players | Add Redis for session cache and global pub/sub; read replicas for PostgreSQL armory queries |
| 2000+ players | Shard by dungeon zone — each zone server owns a subset of rooms; Redis routes cross-zone events |

### Scaling Priorities

1. **First bottleneck: PostgreSQL write contention.** Each tick should not write to the DB. Batch saves via a "dirty entity" queue, or use Redis as a write-through cache with periodic PostgreSQL flush.
2. **Second bottleneck: Global chat fan-out.** A single `tokio::broadcast` channel degrades if thousands of players receive every global message. Upgrade path: Redis pub/sub with per-session subscription tasks.

---

## Anti-Patterns

### Anti-Pattern 1: Shared Mutable World State Behind a Mutex

**What people do:** Wrap the game world in `Arc<Mutex<World>>` and lock it from session tasks.

**Why it's wrong:** Under load, lock contention serializes all players. Rust makes this feel safe but it kills concurrency. Deadlock risk when multiple locks are held simultaneously (e.g., room + combat + inventory).

**Do this instead:** One owner per resource. The World Actor owns the ECS. Session actors communicate by sending messages. No shared mutexes on hot paths.

### Anti-Pattern 2: Writing to PostgreSQL Every Tick

**What people do:** Persist every state change (health, position, inventory) immediately to the database.

**Why it's wrong:** At 20 ticks/sec with 100 players, that's 2000+ DB writes per second before any game logic. PostgreSQL handles this at small scale but it burns IOPS unnecessarily.

**Do this instead:** Keep authoritative state in the ECS + Redis. Mark entities dirty. Flush to PostgreSQL on player disconnect, floor reset, or a periodic save interval (every 30-60 seconds).

### Anti-Pattern 3: Putting Game Logic in the Protocol Handler

**What people do:** Handle "go north" directly in the WebSocket receive callback, touching world state inline.

**Why it's wrong:** Mixes concerns, prevents tick-based ordering, makes combat resolution non-deterministic, breaks replay/testing.

**Do this instead:** Protocol handler only parses and validates input format, then sends a typed command to the World Actor's queue. All logic happens in the tick loop.

### Anti-Pattern 4: Dungeon Generation at Connection Time

**What people do:** Generate a dungeon floor procedurally when the first player steps into it, blocking on the calling task.

**Why it's wrong:** Generation can take 5-50ms for complex floors. Blocking the world tick loop stalls all players.

**Do this instead:** Pre-generate and cache floors at server startup or in a background task. If truly on-demand, run in `tokio::task::spawn_blocking` and load the result asynchronously. Store generated floors in PostgreSQL with their seed so they are stable across reboots.

### Anti-Pattern 5: Embedding Armory Logic in the Game Engine

**What people do:** Have the game engine answer HTTP requests for character profiles, creating coupling between live game state and read queries.

**Why it's wrong:** The armory is a read-heavy, latency-tolerant use case. Routing it through the game engine adds unnecessary load to the tick loop and creates availability coupling.

**Do this instead:** Armory reads PostgreSQL directly through a separate Axum router. Characters update their DB representation on save events. The armory is a read replica consumer, not a game engine query.

---

## Build Order Implications

The component dependency graph suggests this build order:

```
1. shared/protocol       — No deps; defines the message contract everything else compiles against
        │
2. dungeon-gen/          — No server deps; can be built and tested in isolation
        │
3. server/ core          — Network layer (TCP+WS) → Session Actor → World Actor stub → ECS world
        │                  PostgreSQL schema + sqlx queries → Auth module
        │
4. client-native/        — Depends on shared/protocol; can be developed against a stubbed server
        │
5. server/ game systems  — Combat, movement, chat, spawn — built on top of working ECS + World tick
        │
6. dungeon-gen/ integration — Wire generator into server; seed generation + persistence
        │
7. web/terminal          — xterm.js client; should work against the same server as native client
        │
8. web/armory            — REST API routes in Axum + armory SPA; last because it requires stable DB schema
```

Each stage produces a working, runnable artifact. Stage 3 produces a server you can connect to with netcat. Stage 4 produces a usable native client. Stages 5-8 layer in features.

---

## Sources

- [Actors with Tokio — Alice Ryhl](https://ryhl.io/blog/actors-with-tokio/) — Canonical Rust actor pattern, MEDIUM-HIGH confidence
- [MuOxi GitHub — Rust MUD engine with Tokio + Diesel](https://github.com/duysqubix/MuOxi) — Direct precedent for Rust MUD architecture, MEDIUM confidence
- [Evennia Channels Documentation](https://www.evennia.com/docs/latest/Components/Channels.html) — Channel/chat architecture reference from mature Python MUD framework, MEDIUM confidence
- [MUD Standards — WebSocket for MUDs](https://mudstandards.org/websocket/) — Protocol compatibility standards, MEDIUM confidence
- [Rust Forum: Async Game Server Design](https://users.rust-lang.org/t/tokio-tungstenite-async-game-server-design/65996) — Community patterns for tokio-based game servers, MEDIUM confidence
- [Nakama Authoritative Multiplayer](https://heroiclabs.com/docs/nakama/concepts/multiplayer/authoritative/) — Tick-based authoritative server architecture, MEDIUM confidence
- [Procedural Level Generation with Rust](https://www.jamesbaum.co.uk/blether/procedural-level-generation-rust/) — Dungeon pipeline in Rust, MEDIUM confidence

---
*Architecture research for: MUD game server (MUT Remastered)*
*Researched: 2026-03-23*
