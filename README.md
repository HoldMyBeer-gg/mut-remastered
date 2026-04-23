# MUT Remastered

A modern Multi-User Text dungeon (MUD) built in Rust. Persistent shared world, D&D-flavored mechanics, and a Unicode/truecolor TUI that runs in iTerm2 and xterm. Two clients: a native Ratatui terminal client, and a browser client via xterm.js.

Social interaction and exploration first, combat second.

## Status

All seven roadmap phases have landed at least once (server foundation → world & movement → character & combat → native TUI → chat/social → browser client → procedural dungeons). Recent work has been iterating on the native TUI: first-person DDA raycasting for dungeons, 3D mesh renderer, 30fps render loop, class abilities, and a 4-second GCD combat round.

Known rough edges:
- `.planning/ROADMAP.md`'s progress table is stale (says phases 3–7 not started); the commit log is authoritative.
- `web-client/index.html` is a scaffold. The WebSocket endpoint is live, but the browser client doesn't yet encode/decode the postcard binary protocol — the native TUI is the working client today.

## Layout

Cargo workspace (`Cargo.toml`) with three crates plus a web client and content tree:

```
protocol/      shared wire types (postcard, length-prefixed frames, NS_AUTH/NS_WORLD namespace byte)
server/        tokio/axum server — actor-per-session, SQLx+SQLite, ECS-ish world, combat tick loop
client-tui/    ratatui+crossterm native client with split-panel layout, minimap, raycaster
web-client/    static xterm.js page (scaffold)
world/         zone TOML (starting_village, newbie) + data/ (monsters.toml, items.toml)
server/migrations/   SQLx migrations 001–009
.planning/     GSD workflow artifacts (ROADMAP, PROJECT, STATE, phase plans)
```

## Run it

Requires Rust stable (1.87+) and `sqlx-cli` if you want to run migrations manually (the server applies them on startup).

```
cargo run -p server        # starts TCP on 127.0.0.1:4000, WS on 127.0.0.1:4001
cargo run -p client-tui    # native TUI client
```

Open `web-client/index.html` in a browser to see the xterm.js scaffold pointed at `ws://localhost:4001/ws`.

## Configuration

Environment variables (all have defaults — see `server/src/config.rs`):

| Var                | Default                                  |
|--------------------|------------------------------------------|
| `BIND_ADDR`        | `127.0.0.1:4000` (TCP, native client)    |
| `WS_BIND_ADDR`     | `127.0.0.1:4001` (WebSocket, browser)    |
| `DATABASE_URL`     | `sqlite://./mut_remastered.db?mode=rwc`  |
| `SESSION_TTL_SECS` | `604800` (7 days)                        |
| `MUT_WORLDS_DIR`   | `../world/zones`                         |
| `RUST_LOG`         | `server=debug,tower_http=debug`          |

A local `.env` is gitignored; copy-paste the defaults above if you need one.

## Architecture notes

- **Actor-per-session** over a shared `AppState`. No monolithic game loop — each connection is its own task using `tokio::select!` over the socket and its room's `broadcast::channel`.
- **Shared protocol crate.** A message-type change in `protocol/` breaks both clients at compile time. Wire format is postcard with a 4-byte LE length prefix and a leading namespace byte (`NS_AUTH` / `NS_WORLD`) to prevent cross-decode collisions.
- **World state.** Zones are authored as TOML under `world/zones/<zone>/zone.toml`; runtime state (positions, triggered flags) is overlaid from SQLite on load.
- **Auth.** Argon2id via `spawn_blocking` so hashing doesn't stall the async runtime; UUID session tokens (not JWT) for TCP sessions.
- **Combat.** Background tick loop at 4s rounds, GCD bar in the TUI, per-room monster spawn tables, respawn timers.
- **Procedural dungeons.** BSP generation with connectivity guarantees; boss rooms and hand-crafted set pieces injected at anchor points. Accessible in-game via the market-square dungeon entrance; respawn falls back to market square if a saved room has despawned.

## Testing

```
cargo test --workspace
```

Integration tests run against in-memory SQLite (`sqlite:file:testdb_UUID?mode=memory&cache=shared`) for isolation.
