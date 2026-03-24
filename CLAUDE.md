<!-- GSD:project-start source:PROJECT.md -->
## Project

**MUT Remastered**

A modern Multi-User Text dungeon (MUD) built in Rust with a persistent world, D&D-flavored mechanics, and a rich Unicode TUI that runs in standard terminals (iTerm2, xterm). Players can connect via a native terminal client or a browser-based TUI (xterm.js). The web frontend also serves as a character armory and server hub.

**Core Value:** Players can explore a shared persistent dungeon world together through a beautiful terminal interface — social interaction and exploration come first, combat second.

### Constraints

- **Terminal compatibility**: Must render correctly in iTerm2 and xterm using Unicode/truecolor — no terminal-specific protocols
- **Language**: Rust for server and native TUI client
- **Web client**: Browser-based TUI via xterm.js, web app for armory/hub
- **3D assets**: Use blend-ai for any model generation
- **Rules system**: D&D-flavored but simplified — not a strict SRD implementation
<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->
## Technology Stack

## Recommended Stack
### Core Technologies
| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| Rust (stable) | 1.87+ | Server + native TUI client language | Required by project constraints; safety guarantees prevent entire classes of game-state corruption bugs common in C++ MUD engines |
| Tokio | 1.48 | Async runtime for server | De facto standard with 437M+ downloads; canonical choice for networked Rust services; Axum, SQLx, Tracing all assume it |
| Axum | 0.8.x (0.8.8) | HTTP + WebSocket server | Built by the Tokio team; native WebSocket upgrade (`axum::extract::ws`) handles the xterm.js browser client without a separate WebSocket layer; 0.8 released Jan 2025 with ergonomic improvements |
| SQLx | 0.8.6 | Async database access with compile-time query validation | Pure async, no ORM lock-in, compile-time checked SQL macros catch mistakes before deploy; supports both SQLite (dev/small) and PostgreSQL (prod) with the same code |
| Ratatui | 0.30.0 | Native terminal TUI | The actively-maintained fork of tui-rs (2023+); 11.9M downloads; 0.30 (Dec 2024) is the largest release ever with modular architecture and no_std support |
| Crossterm | 0.29.0 | Terminal backend for Ratatui | Default Ratatui backend; cross-platform (macOS/Linux/Windows), works in iTerm2 and xterm without terminal-specific protocols; pure Rust |
### Web Client Technologies
| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| xterm.js | 6.0.0 | Browser-based terminal emulator | Industry standard (used by VS Code, GitHub); Dec 2024 v6 release adds synchronized output (DEC mode 2026) and shadow DOM WebGL support; `@xterm/addon-attach` connects directly to the axum WebSocket endpoint |
| @xterm/addon-attach | 0.12.0 | WebSocket bridge for xterm.js | Official addon; connects xterm.js terminal I/O to a WebSocket stream — your Axum server speaks raw ANSI sequences to this addon over WebSocket, making the web client functionally identical to the native client |
| @xterm/addon-fit | (latest) | Responsive terminal sizing | Resizes the xterm.js canvas to fill its container; required for any real-world terminal embedding |
| @xterm/addon-web-links | (latest) | Clickable URLs in web terminal | Minor quality-of-life; easily loadable as addon |
| SvelteKit | 2.x (2.55.0) | Web armory / account hub framework | Smaller bundles than React/Next.js; performance-first; Threlte (Three.js wrapper) integrates cleanly for 3D model display; straightforward SPA routing for armory profiles |
| Threlte | 8.x | 3D model rendering in browser | Svelte-native Three.js wrapper; declarative syntax for displaying blend-ai generated GLB/GLTF 3D models in the armory; actively maintained at threlte.xyz |
| Three.js | r170+ | 3D rendering engine (via Threlte) | Standard for browser 3D; required by Threlte; supports GLTF/GLB which is the output format for blend-ai generated models |
### Serialization and Networking
| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| serde | 1.0 | Serialization framework | Universal in Rust ecosystem; all major crates derive Serialize/Deserialize from it |
| serde_json | 1.0 | JSON for REST API / web armory | Human-readable protocol for the web armory API; straightforward Axum integration |
| bincode | 2.x | Binary encoding for native TUI ↔ server protocol | Fastest Rust serialization format; use for the native client ↔ server protocol where bandwidth and latency matter; not for browser-facing endpoints |
| tokio-tungstenite | 0.27.0 | Low-level WebSocket (native client) | The native TUI client uses this directly for its WebSocket connection; the server side uses Axum's built-in ws (which wraps tungstenite internally) |
### Database
| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| SQLite (via SQLx) | SQLx 0.8.6 | Development and small-scale deployments | Zero-config; all world state in one file; SQLx's connection pool handles the async access correctly; easy backup |
| PostgreSQL (via SQLx) | SQLx 0.8.6 | Production deployments | Same SQLx query code works for both; LISTEN/NOTIFY available for future pub-sub needs; proven for persistent game worlds with high write loads |
### Authentication and Security
| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| argon2 | 0.5 | Password hashing | Memory-hard; current best practice over bcrypt/PBKDF2; official recommendation in OWASP 2025 |
| jsonwebtoken | 9.x | Session tokens for web armory | Standard JWT for the web HTTP API; keep short-lived access tokens + long-lived refresh tokens stored server-side in the DB |
| tower-http | 0.6 | Middleware for Axum (CORS, compression, rate limiting) | Integrates natively with Axum; CORS required for the SvelteKit armory calling the Axum API from a different origin |
### Procedural Generation
| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| noise | 0.9 | Perlin/Simplex/Worley noise for terrain and dungeon generation | The standard Rust noise library; supports Perlin, Value, Worley; composable NoiseFn modules for complex results |
| rand | 0.9 | Random number generation | Designed-for-Rust RNG; more ergonomic and featureful than `random()`; use for dice rolls, loot tables, enemy placement |
### Observability
| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| tracing | 0.1 | Structured async-aware logging | Tokio project; designed specifically for async where log lines from concurrent tasks interleave; spans track causality across task boundaries — critical for debugging multiplayer game state |
| tracing-subscriber | 0.3 | Log output formatting | Provides FmtSubscriber for dev and JSON output for production log aggregation |
## Development Tools
| Tool | Purpose | Notes |
|------|---------|-------|
| cargo-watch | Auto-rebuild on file changes | Run `cargo watch -x run` during development |
| sqlx-cli | Run SQLx migrations, generate offline query cache | Required for `SQLX_OFFLINE=true` in CI where no DB is available at build time |
| cargo-nextest | Faster test runner | Parallelizes test execution; useful once the game logic test suite grows |
| Vite | SvelteKit dev server and bundler | Comes with SvelteKit; SvelteKit 2.55+ supports Vite 7/Rolldown |
## Installation
# Rust toolchain (stable)
# SQLx CLI for migrations
# cargo-watch for hot reload in development
# Server Cargo.toml dependencies
# [dependencies]
# tokio = { version = "1", features = ["full"] }
# axum = { version = "0.8", features = ["ws"] }
# sqlx = { version = "0.8", features = ["sqlite", "postgres", "runtime-tokio-native-tls", "macros"] }
# serde = { version = "1", features = ["derive"] }
# serde_json = "1"
# bincode = "2"
# tokio-tungstenite = "0.27"
# tracing = "0.1"
# tracing-subscriber = { version = "0.3", features = ["env-filter"] }
# tower-http = { version = "0.6", features = ["cors", "compression-gzip"] }
# argon2 = "0.5"
# jsonwebtoken = "9"
# rand = "0.9"
# noise = "0.9"
# Native TUI client Cargo.toml dependencies
# [dependencies]
# ratatui = "0.30"
# crossterm = "0.29"
# tokio = { version = "1", features = ["full"] }
# tokio-tungstenite = "0.27"
# serde = { version = "1", features = ["derive"] }
# bincode = "2"
# Web client (SvelteKit)
## Alternatives Considered
| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| Axum 0.8 | Actix-web | When raw throughput benchmarks are the deciding factor and your team is comfortable with Actix's actor model; Axum is preferred here because it shares the Tokio stack with everything else and its built-in ws module eliminates a dependency |
| Axum 0.8 | Warp | Never for new projects; Warp's filter combinator DX is significantly worse than Axum's extractor model; actively losing community share |
| SQLx | SeaORM / Diesel | SeaORM if you want full ORM abstractions and are comfortable with async-first ORM patterns; Diesel if you want synchronous ORM with the strongest compile-time guarantees; SQLx is preferred here because it stays close to SQL and avoids ORM impedance mismatch with complex game queries |
| SQLite → PostgreSQL | MongoDB | Only if the data model is genuinely document-centric; game world state (rooms, characters, items, chat) is relational; SQL joins for inventory/equipment are cleaner than document lookups |
| Ratatui + Crossterm | Cursive | Cursive has a widget abstraction layer that's easier for pure form-based UIs; Ratatui's immediate-mode rendering is better for game UIs that need to re-render the whole screen frequently (dungeon maps, combat log, chat pane all updating at once) |
| SvelteKit | Next.js (React) | Next.js if the team is already React-heavy or needs a massive ecosystem of UI component libraries; SvelteKit's smaller bundles and Threlte's clean Three.js integration are the deciding factors for an armory with 3D models |
| Threlte | React Three Fiber | React Three Fiber if using Next.js; equivalent capability to Threlte but for React |
| bincode | MessagePack (rmp-serde) | MessagePack if the native client protocol must be interoperable with non-Rust implementations or needs human-inspectable bytes; bincode is faster and sufficient for a Rust-on-both-ends connection |
| tokio broadcast::channel | Redis pub/sub | Redis pub/sub when the server needs to scale horizontally across multiple processes/machines; for a single-process MUD server tokio's in-process broadcast channels are simpler and have zero latency |
## What NOT to Use
| Avoid | Why | Use Instead |
|-------|-----|-------------|
| tui-rs | Archived in 2023; development ceased; Ratatui is the official fork | Ratatui 0.30 |
| termion | Linux/macOS only; no Windows support; the native TUI client should work anywhere contributors develop | Crossterm 0.29 |
| Rocket | Version churn historically caused breaking changes on minor versions; requires nightly Rust features in older versions; Axum is more stable and has equivalent DX in 2025 | Axum 0.8 |
| Diesel (for async server) | Diesel is synchronous; using it in an async Tokio context requires blocking thread pools via `spawn_blocking`, negating async benefits for a server handling hundreds of concurrent players | SQLx 0.8 |
| bcrypt / PBKDF2 | Not memory-hard; vulnerable to GPU-accelerated cracking; OWASP recommends Argon2id as first choice | argon2 crate (Argon2id) |
| ws (crate) | Unmaintained; tungstenite/tokio-tungstenite is the ecosystem standard | tokio-tungstenite 0.27 |
| xterm npm package (unscoped) | The old `xterm` npm package (5.3.0) is 3 years old; the project migrated to the `@xterm/` scope; `@xterm/xterm` 6.0 is the current package | `@xterm/xterm` 6.0 |
| GPU-accelerated terminal renderers (Alacritty renderer, Kitty protocol) | Explicitly out of scope; xterm.js does not support terminal-specific GPU protocols; breaks browser client parity | Standard Unicode/ANSI via xterm.js + crossterm |
## Stack Patterns by Variant
- All world state in one Tokio process with Arc<RwLock<GameWorld>>
- tokio::sync::broadcast::channel per room/zone for chat fan-out
- tokio::sync::mpsc::channel for player command ingestion
- Because: Simplest correct starting point; avoids distributed systems complexity before the game mechanics are stable
- Introduce Redis pub/sub for cross-process broadcast when a single process hits CPU limits
- Because: Premature for v1; game logic is CPU-cheap compared to I/O
- tokio-tungstenite WebSocket → bincode-encoded game protocol messages
- Because: Rust-to-Rust; bincode is the fastest option, no interop requirements
- xterm.js @xterm/addon-attach → Axum WebSocket → raw ANSI escape sequences
- Because: xterm.js renders ANSI natively; the server writes to the browser client the same way crossterm writes to the native terminal — no separate protocol layer needed
- Start with SQLite for development and single-server deployments
- SQLx feature flags make switching to PostgreSQL a one-line `Cargo.toml` change
- Because: SQLite requires zero infrastructure; ship the game, then migrate if needed
## Version Compatibility
| Package | Compatible With | Notes |
|---------|-----------------|-------|
| ratatui 0.30 | crossterm 0.29 (default feature `crossterm_0_29`) | Ratatui 0.30 modularized backends; use `ratatui-crossterm` crate if pinning separately |
| axum 0.8 | tokio 1.x, tower 0.5, hyper 1.x | Axum 0.8 upgraded to hyper 1.0; do not mix with crates that depend on hyper 0.x |
| sqlx 0.8 | tokio 1.x via `runtime-tokio-native-tls` feature | Use `runtime-tokio-rustls` if you want to avoid OpenSSL linking issues on Linux |
| tokio-tungstenite 0.27 | tungstenite 0.29, tokio 1.x | tungstenite 0.29 is the paired release |
| @xterm/xterm 6.0 | @xterm/addon-attach 0.12 | v6 removed deprecated `windowsMode` and canvas renderer addon |
| SvelteKit 2.55 | Vite 7, Rolldown | SvelteKit 2.x supports Vite 7; check Threlte compatibility with SvelteKit 2 before upgrading |
| threlte 8.x | three r170+, svelte 5 | Threlte 8 requires Svelte 5; confirm version when scaffolding |
## Sources
- https://github.com/ratatui/ratatui/releases — Ratatui 0.30.0 confirmed Dec 2024 (HIGH confidence)
- https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0 — Axum 0.8 release notes (HIGH confidence)
- https://docs.rs/crate/sqlx/latest — SQLx 0.8.6 confirmed (HIGH confidence)
- https://docs.rs/crate/tokio-tungstenite/latest — tokio-tungstenite 0.26.2/0.27.0 confirmed (HIGH confidence)
- https://github.com/xtermjs/xterm.js/releases — xterm.js 6.0.0 Dec 22 2024 (HIGH confidence)
- https://www.npmjs.com/package/@xterm/addon-attach — @xterm/addon-attach 0.12.0 (HIGH confidence)
- https://crates.io/crates/crossterm — crossterm 0.29.0 Apr 2025 (HIGH confidence)
- https://github.com/sveltejs/kit/releases — SvelteKit 2.55.0 current stable (HIGH confidence)
- https://threlte.xyz — Threlte 8.x for Svelte 5 + Three.js (MEDIUM confidence — verify Svelte 5 requirement before scaffolding)
- https://docs.rs/crate/sqlx/latest — sqlx 0.8.6 confirmed (HIGH confidence)
- https://github.com/Razaekel/noise-rs — noise-rs procedural generation (MEDIUM confidence — verify current version on crates.io)
- https://github.com/duysqubix/MuOxi — MuOxi Rust MUD reference architecture using Tokio + PostgreSQL + Redis (LOW confidence for direct adoption — last active 2020, but architecture patterns remain valid)
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd:quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd:debug` for investigation and bug fixing
- `/gsd:execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd:profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
