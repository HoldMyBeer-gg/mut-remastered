# Phase 1: Server Foundation - Research

**Researched:** 2026-03-23
**Domain:** Rust async networking, authentication, shared protocol crate (Cargo workspace)
**Confidence:** HIGH

---

## Summary

Phase 1 establishes the core server infrastructure that all later phases build on. The primary technical work is: (1) a Cargo workspace with a shared `protocol` crate that the server and both clients compile against; (2) a Tokio/Axum server that accepts TCP connections and maintains per-session actor state; (3) password hashing with Argon2id and session management via JWT or in-memory token store; and (4) an SQLite database via SQLx for persisting accounts and sessions.

The stack described in CLAUDE.md is largely correct as of 2026-03-23, with two important corrections: **bincode is unmaintained** (RUSTSEC-2025-0141, archived August 2025) and must be replaced with `postcard` for the binary protocol; and `tokio-tungstenite` is now at 0.29.0 (not 0.27). All other recommended versions have been verified against crates.io.

**Primary recommendation:** Use a Cargo workspace from day one with members `protocol/` (library), `server/` (binary), and `client-tui/` (binary). The `protocol` crate compiles into both binaries; a message type change produces a compile error in both — satisfying NETW-04 without runtime checks.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

No CONTEXT.md exists for this phase. Constraints are derived from CLAUDE.md project instructions and STATE.md decisions.

### Locked Decisions (from STATE.md and CLAUDE.md)
- Actor-per-session pattern with fixed-tick ECS World Actor chosen (no monolithic loop, no shared-mutex pattern)
- Shared protocol crate (NETW-04) placed in Phase 1 — all downstream phases compile against it
- Rust for server and native TUI client (required, non-negotiable)
- Tokio 1.x as async runtime
- Axum 0.8.x for HTTP + WebSocket
- SQLx 0.8.x with SQLite for development
- argon2 crate (Argon2id) for password hashing
- TCP for native client connections
- Terminal compatibility: must render correctly in iTerm2 and xterm, no terminal-specific protocols

### Claude's Discretion
- Exact workspace layout (crate naming, directory structure)
- Session token implementation detail (JWT vs. in-memory UUID map)
- Error type strategy (thiserror vs. anyhow per crate type)
- Integration test approach

### Deferred Ideas (OUT OF SCOPE for Phase 1)
- WebSocket support for browser clients (Phase 6)
- SvelteKit web armory (v2)
- TUI rendering (Phase 4)
- Character/world/combat systems (Phases 2-5)
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| AUTH-01 | User can create an account with username and hashed password | argon2 Argon2id + SQLx account table; PHC string format stored in DB |
| AUTH-02 | User can log in and receive a persistent session | Session token (JWT or UUID) issued at login; stored in DB or in-memory map; returned to client per protocol message |
| AUTH-08 | User can log out cleanly with automatic save | Session invalidation on logout command; character state flush to SQLite before connection drop |
| NETW-01 | Game server handles multiple concurrent player connections via TCP | TcpListener accept loop + tokio::spawn per connection; no blocking between players |
| NETW-04 | Shared protocol crate ensures message consistency between native and web clients | Cargo workspace with `protocol` library crate; server and client binaries both depend on it; serde + postcard derive on all message types |
</phase_requirements>

---

## Project Constraints (from CLAUDE.md)

Directives the planner MUST verify compliance with:

- Language: Rust (stable, 1.87+). Current installed: rustc 1.92.0.
- Async runtime: Tokio 1.x (current verified: 1.50.0)
- HTTP/WebSocket server: Axum 0.8.x (current verified: 0.8.8)
- Database: SQLx 0.8.x (current verified: 0.8.6) with SQLite for dev
- Password hashing: argon2 crate, Argon2id variant (current verified: 0.5.3)
- Session tokens: jsonwebtoken 9.x — **CORRECTION: current is 10.x with breaking API change** (see Standard Stack)
- Binary protocol: bincode 2.x — **CORRECTION: bincode is unmaintained (RUSTSEC-2025-0141)** (see Standard Stack)
- TUI: Ratatui 0.30 + Crossterm 0.29 (Phase 4, not Phase 1)
- No terminal-specific protocols; no GPU-accelerated renderers
- Do NOT use: tui-rs, termion, Rocket, Diesel (async), bcrypt/PBKDF2, ws crate, unscoped xterm npm package

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tokio | 1.50.0 | Async runtime | Required by CLAUDE.md; de facto standard; all other crates assume it |
| axum | 0.8.8 | HTTP + WebSocket routing | Required by CLAUDE.md; built by Tokio team; native ws upgrade |
| sqlx | 0.8.6 | Async DB access with compile-time query validation | Required by CLAUDE.md; SQLite + PostgreSQL with same code |
| argon2 | 0.5.3 | Argon2id password hashing | Required by CLAUDE.md; OWASP 2025 first choice |
| serde | 1.0.228 | Serialization derive | Universal in Rust ecosystem; required by protocol crate |
| serde_json | 1.0.149 | JSON encoding for REST responses | Required by CLAUDE.md; human-readable API protocol |
| postcard | 1.1.3 | Binary encoding for native TUI protocol | **Replaces bincode** (unmaintained); serde-compatible; stable wire format since 1.0; 25M+ downloads; actively maintained |
| tracing | 0.1.44 | Structured async-aware logging | Required by CLAUDE.md; spans track causality across concurrent tasks |
| tracing-subscriber | 0.3.23 | Log output formatting | Required by CLAUDE.md; FmtSubscriber for dev |
| tower-http | 0.6.8 | CORS, compression middleware for Axum | Required by CLAUDE.md |
| thiserror | 2.0.18 | Typed error enums for library crates | Standard for crates exposing public error types (protocol, domain layers) |
| anyhow | 1.0.102 | Error propagation for binary crates | Standard for application-level code (server main, CLI) |
| uuid | 1.22.0 | Session token IDs / account IDs | Standard identifier generation |
| chrono | 0.4.44 | JWT expiry timestamps | Time handling |

### Session Tokens
| Library | Version | Purpose | Notes |
|---------|---------|---------|-------|
| jsonwebtoken | 10.3.0 | JWT session tokens for web armory (future) | **v10 breaking change:** must select crypto backend via feature flag — use `features = ["aws_lc_rs"]` OR `features = ["rust_crypto"]`; for Phase 1 (TCP-only, no web), prefer simple UUID session tokens stored in DB |

> **Phase 1 recommendation:** For TCP native client sessions, use a UUID session token stored in the DB `sessions` table rather than JWT. JWT is stateless and suited for the HTTP/web armory in later phases. A DB-backed session is simpler, revocable instantly on logout (AUTH-08), and eliminates a crypto backend decision. Introduce jsonwebtoken when the web armory is built.

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tokio-tungstenite | 0.29.0 | WebSocket for native client (Phase 6) | Not needed in Phase 1; Phase 1 uses raw TCP |
| axum-test | 19.1.1 | Integration testing of Axum routes | Use in test suites for auth endpoints |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| postcard | bitcode 0.6.9 | bitcode is faster but no_std-only, fewer docs, smaller community (4.5M vs 25M downloads) |
| postcard | rmp-serde (MessagePack) | MessagePack if non-Rust clients need to read the binary protocol; postcard if Rust-to-Rust only |
| UUID session tokens | JWT (jsonwebtoken) | JWT for Phase 6+ web clients (stateless HTTP); UUID for Phase 1 TCP (simpler, instantly revocable) |

**Installation (server Cargo.toml):**
```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
axum = { version = "0.8", features = ["ws"] }
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio-native-tls", "macros", "migrate"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
postcard = { version = "1", features = ["alloc"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tower-http = { version = "0.6", features = ["cors", "compression-gzip"] }
argon2 = "0.5"
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"
anyhow = "1"
```

**Installation (protocol crate Cargo.toml — library):**
```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
postcard = { version = "1", features = ["alloc"] }
```

**Version verification:** All versions verified against crates.io API on 2026-03-23.

---

## Architecture Patterns

### Recommended Workspace Structure
```
mut_remastered/          # workspace root
├── Cargo.toml           # [workspace] members = ["protocol", "server", "client-tui"]
├── Cargo.lock           # shared lock file
├── protocol/            # library crate — shared message types
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── auth.rs      # Login, Register, Logout messages
│       └── game.rs      # Placeholder for Phase 2+ game messages
├── server/              # binary crate — game server
│   ├── Cargo.toml
│   ├── migrations/      # SQLx migration files (*.sql)
│   └── src/
│       ├── main.rs      # Tokio entry point
│       ├── config.rs    # Server config (port, DB URL)
│       ├── db.rs        # SQLx pool setup + migration runner
│       ├── auth/
│       │   ├── mod.rs
│       │   ├── hash.rs  # Argon2id hash/verify
│       │   └── session.rs # Session create/lookup/delete
│       ├── net/
│       │   ├── mod.rs
│       │   └── listener.rs # TcpListener accept loop
│       └── session/
│           ├── mod.rs
│           └── actor.rs # Per-connection actor (mpsc receiver)
└── client-tui/          # binary crate — native TUI (minimal stub for Phase 1)
    ├── Cargo.toml
    └── src/
        └── main.rs      # Connects, sends Register/Login, reads response
```

### Pattern 1: Cargo Workspace with Shared Protocol Crate (NETW-04)

**What:** A workspace root `Cargo.toml` declares `protocol/` as a library member. Both `server/` and `client-tui/` list `protocol = { path = "../protocol" }` in their dependencies. All message types are defined once in `protocol` with `#[derive(Serialize, Deserialize)]`. A type change in `protocol` fails compilation in any crate that uses that type.

**When to use:** Always; this is the foundational structure for the entire project.

**Example:**
```toml
# Cargo.toml (workspace root)
[workspace]
resolver = "2"
members = ["protocol", "server", "client-tui"]

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
postcard = { version = "1", features = ["alloc"] }
tokio = { version = "1", features = ["full"] }
```

```rust
// protocol/src/auth.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMsg {
    Register { username: String, password: String },
    Login { username: String, password: String },
    Logout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMsg {
    RegisterOk { account_id: String },
    LoginOk { session_token: String },
    LogoutOk,
    Error { code: ErrorCode, message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorCode {
    InvalidCredentials,
    UsernameTaken,
    SessionExpired,
}
```

### Pattern 2: Actor-per-Connection with Tokio mpsc (NETW-01)

**What:** Each accepted TCP connection spawns two tasks: a reader task that reads bytes off the socket and forwards deserialized `ClientMsg` to an mpsc channel, and the connection actor task that owns session state and processes messages from the channel. Source: Alice Ryhl, "Actors with Tokio" (ryhl.io/blog/actors-with-tokio/).

**When to use:** Any time per-connection state must be mutable without a shared mutex. Satisfies the locked decision from STATE.md.

**Example:**
```rust
// server/src/net/listener.rs
pub async fn run_listener(addr: &str, state: Arc<AppState>) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (socket, peer_addr) = listener.accept().await?;
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, peer_addr, state).await {
                tracing::warn!(%peer_addr, error = %e, "connection closed with error");
            }
        });
    }
}
```

**Key insight:** Each `tokio::spawn` gets its own stack; a panic or slow operation in one task does not block others. This directly satisfies NETW-01 (multiple concurrent players).

### Pattern 3: Argon2id Password Hashing (AUTH-01)

**What:** Use `argon2::Argon2::default()` which uses Argon2id variant with OWASP minimum parameters (m=19456 KiB, t=2, p=1). Hash on register; verify on login. Store the PHC string in the DB (includes salt and parameters embedded).

**Example:**
```rust
// server/src/auth/hash.rs
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{rand_core::OsRng, SaltString};

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default(); // Argon2id v19, m=19456, t=2, p=1
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("hash error: {e}"))?;
    Ok(hash.to_string()) // PHC string format: $argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>
}

pub fn verify_password(password: &str, phc_hash: &str) -> anyhow::Result<bool> {
    let parsed_hash = PasswordHash::new(phc_hash)
        .map_err(|e| anyhow::anyhow!("invalid hash: {e}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}
```

### Pattern 4: SQLx Migrations + Compile-Time Query Validation

**What:** Place `.sql` files under `server/migrations/`. Call `sqlx::migrate!()` at startup. Use `sqlx::query!()` macros for compile-time checked SQL — requires a `.sqlx/` query cache (generated by `cargo sqlx prepare`) or a live DB at compile time.

**Example:**
```sql
-- server/migrations/001_accounts.sql
CREATE TABLE IF NOT EXISTS accounts (
    id          TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    username    TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at  INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE IF NOT EXISTS sessions (
    token       TEXT PRIMARY KEY,
    account_id  TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    created_at  INTEGER NOT NULL DEFAULT (unixepoch()),
    expires_at  INTEGER NOT NULL
);
```

```rust
// server/src/db.rs
pub async fn init_db(database_url: &str) -> anyhow::Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
```

**Warning:** `sqlx::query!()` macros require either a live DB at `DATABASE_URL` compile time or `SQLX_OFFLINE=true` with a cached `.sqlx/` directory. Plan for this in CI.

### Anti-Patterns to Avoid

- **Shared `Arc<Mutex<HashMap<...>>>` for per-player state:** Locks the entire map for every message. Use the actor-per-connection pattern instead (locked decision in STATE.md).
- **Blocking password hashing on the Tokio thread:** Argon2 is CPU-intensive. Wrap in `tokio::task::spawn_blocking` to prevent blocking the async runtime.
- **Storing raw passwords or MD5/SHA256 hashes:** OWASP 2025 forbids this. Store only PHC strings from argon2 crate.
- **JWT for Phase 1 TCP sessions:** Stateless JWT is designed for HTTP. For TCP sessions that must be instantly revocable on logout (AUTH-08), use a UUID token stored in the `sessions` table.
- **Single `Cargo.toml` flat structure:** Prevents compile-time protocol enforcement (NETW-04). Use workspace from the start.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Password hashing | Custom bcrypt/PBKDF2/SHA256 | argon2 crate (Argon2id) | GPU-crackable without memory-hardness; subtle implementation bugs; OWASP 2025 mandates Argon2id |
| Binary serialization | Custom byte packing | postcard (serde-compatible) | Endianness bugs, alignment issues, versioning nightmares; postcard has stable wire format spec |
| SQL query construction | String concatenation | sqlx query macros | SQL injection; no compile-time checking; missing parameter binding |
| DB connection pooling | Single connection + mutex | sqlx SqlitePool | Connection starvation, no timeout handling, no retry logic |
| Async TCP accept loop | Custom epoll/select | tokio TcpListener | Portability; correct wakeup semantics; works with Tokio scheduler |
| Per-connection message dispatch | Thread-per-connection + channels | tokio::spawn + mpsc | 10K+ threads is infeasible; tokio tasks are green threads at ~KB each |

**Key insight:** Authentication and binary protocol are exactly the categories where "it's simple, I'll write it" leads to security vulnerabilities and compatibility breaks. Use the ecosystem.

---

## Common Pitfalls

### Pitfall 1: SQLx Compile-Time Macros Without DB or Cache

**What goes wrong:** `sqlx::query!()` fails to compile with "DATABASE_URL must be set" or "query not found in offline cache."
**Why it happens:** The macros need to validate SQL against a real schema at compile time.
**How to avoid:** During development, set `DATABASE_URL=sqlite://./dev.db` in `.env`. For CI without a DB: run `cargo sqlx prepare` to generate `.sqlx/` cache and set `SQLX_OFFLINE=true`. Commit `.sqlx/` to git.
**Warning signs:** Build fails in CI but works locally.

### Pitfall 2: Blocking Argon2 on Tokio Thread

**What goes wrong:** Server hangs under load; all connections slow down when one player logs in.
**Why it happens:** Argon2 hashing takes ~100ms+ and blocks the async thread, preventing other tasks from running.
**How to avoid:** Always wrap hash/verify in `tokio::task::spawn_blocking(|| ...)`.
**Warning signs:** Server response times spike during login; `cargo flamegraph` shows Argon2 on the main thread.

### Pitfall 3: bincode Dependency

**What goes wrong:** `cargo audit` fails with RUSTSEC-2025-0141 unmaintained advisory; CI blocks deployment.
**Why it happens:** bincode was archived August 2025; advisory issued December 2025.
**How to avoid:** Use `postcard` instead. It is serde-compatible, has a stable wire format spec, and is actively maintained. The API is nearly identical: `postcard::to_allocvec(&msg)?` / `postcard::from_bytes(&bytes)?`.
**Warning signs:** CLAUDE.md references bincode 2.x — this is outdated. Treat it as already corrected.

### Pitfall 4: Workspace resolver Version

**What goes wrong:** Feature unification causes unexpected behavior or compile errors when mixing crates.
**Why it happens:** Rust 2021 edition workspaces default to resolver = "2" but if omitted the old resolver is used.
**How to avoid:** Explicitly set `resolver = "2"` in the workspace `Cargo.toml`. With Rust 1.85+ and edition 2024 you get resolver 3, which is even better for feature isolation.
**Warning signs:** A crate gets a feature enabled that its `Cargo.toml` doesn't request.

### Pitfall 5: Session Token Not Invalidated on Logout

**What goes wrong:** AUTH-08 fails — player logs out but their session token still works.
**Why it happens:** JWT is stateless (can't be revoked without a denylist); or the DB delete is not awaited before the connection closes.
**How to avoid:** Use UUID tokens in a DB `sessions` table. On `Logout` message: delete the row, flush pending character state (write to accounts/characters table), then close the connection. The delete must complete before responding with `LogoutOk`.
**Warning signs:** Integration test shows token valid after logout response.

### Pitfall 6: tokio-tungstenite Version Mismatch

**What goes wrong:** Dependency conflict between axum (which embeds tungstenite) and tokio-tungstenite direct dependency.
**Why it happens:** Axum 0.8 uses tungstenite internally; if you add tokio-tungstenite at a different tungstenite version, Cargo may complain or silently link two versions.
**How to avoid:** Phase 1 does not need tokio-tungstenite at all (TCP only). When added in Phase 6, use tokio-tungstenite 0.29.0 and verify it aligns with axum's transitive tungstenite version using `cargo tree`.
**Warning signs:** `cargo tree` shows two versions of tungstenite.

---

## Code Examples

### postcard Encode/Decode (replaces bincode)
```rust
// Source: https://docs.rs/postcard/latest/postcard/
use postcard::{from_bytes, to_allocvec};
use protocol::auth::{ClientMsg, ServerMsg};

// Encode
let msg = ClientMsg::Login {
    username: "alice".to_string(),
    password: "hunter2".to_string(),
};
let bytes: Vec<u8> = to_allocvec(&msg)?;

// Decode
let decoded: ClientMsg = from_bytes(&bytes)?;
```

### Axum Route Setup with State
```rust
// Source: https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0
use axum::{Router, routing::get};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::SqlitePool,
}

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .with_state(state)
}
```

### SQLx Query with Compile-Time Check
```rust
// Source: https://docs.rs/sqlx/latest/sqlx/
let account = sqlx::query!(
    "SELECT id, username, password_hash FROM accounts WHERE username = ?",
    username
)
.fetch_optional(&pool)
.await?;
```

### spawn_blocking for Argon2
```rust
// Prevents blocking the Tokio async thread
let password = password.to_string();
let hash = phc_hash.to_string();
let verified = tokio::task::spawn_blocking(move || verify_password(&password, &hash))
    .await??;
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| bincode 2.x for binary serialization | postcard 1.x | RUSTSEC-2025-0141, archived Aug 2025 | Replace in Cargo.toml; API nearly identical |
| tokio-tungstenite 0.27 | tokio-tungstenite 0.29 | 2025 | Update version in Cargo.toml when needed (Phase 6) |
| jsonwebtoken 9.x | jsonwebtoken 10.x (requires crypto backend feature) | 2025 | Add `features = ["rust_crypto"]` or `["aws_lc_rs"]` when introduced |
| thiserror 1.x | thiserror 2.x | 2024 | New derive API; do not mix 1.x and 2.x in same workspace |
| Rust 2021 edition + resolver = "2" | Rust 2024 edition + resolver = "3" available | Rust 1.85+ | Optional; use 2024 edition for cleaner feature isolation |

**Deprecated/outdated (from CLAUDE.md — confirmed):**
- `bincode`: Unmaintained (RUSTSEC-2025-0141). Use `postcard`.
- `tui-rs`: Archived 2023. Use `ratatui 0.30`.
- `termion`: Linux/macOS only. Use `crossterm 0.29`.

---

## Open Questions

1. **Argon2 hashing parameters under load**
   - What we know: Default Argon2id params (m=19456, t=2) take ~100ms on typical hardware
   - What's unclear: Whether the target server hardware requires tuning (higher memory for security, lower for throughput)
   - Recommendation: Use defaults for Phase 1; add a benchmark in a later phase if login latency becomes a concern

2. **SQLx offline mode setup in CI**
   - What we know: CI will not have a running SQLite DB unless we create one; `cargo sqlx prepare` generates `.sqlx/` cache
   - What's unclear: Whether the project CI (GitHub Actions or similar) is configured
   - Recommendation: Plan a Wave 0 task to run `cargo sqlx prepare` and commit `.sqlx/` before the first SQLx query! macro is written

3. **Session expiry policy**
   - What we know: AUTH-02 requires a persistent session; AUTH-08 requires clean logout
   - What's unclear: How long sessions should live (1 hour? 30 days? Until logout?)
   - Recommendation: Implement with a configurable TTL in config.rs; default to 7 days for a game where players return daily

4. **Protocol framing over raw TCP**
   - What we know: postcard produces variable-length byte sequences; TCP is a stream (no message boundaries)
   - What's unclear: Exact framing strategy (length-prefix? delimiter?)
   - Recommendation: Use a 4-byte little-endian length prefix before each postcard message. This is the standard approach and avoids delimiter collisions in binary data.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Rust (stable) | All Rust compilation | Yes | 1.92.0 | — |
| cargo | Build system | Yes | 1.92.0 | — |
| SQLite | Dev database (via SQLx) | Yes (bundled in sqlx `sqlite` feature) | bundled | — |
| sqlx-cli | Migrations, .sqlx cache generation | No | — | Run migrations at server startup only; generate .sqlx manually |
| cargo-watch | Hot reload during development | No | — | Use `cargo run` manually |
| cargo-nextest | Faster test runner | No | — | Use `cargo test` (built-in, always available) |

**Missing dependencies with no fallback:**
- None. All blocking requirements are met by the Rust toolchain and sqlx's bundled SQLite.

**Missing dependencies with fallback:**
- `sqlx-cli`: Install with `cargo install sqlx-cli --no-default-features --features sqlite` before running `cargo sqlx prepare`. Wave 0 task should include this install step.
- `cargo-watch`: Optional; `cargo run` works for Phase 1.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in (`cargo test`) + `#[tokio::test]` for async |
| Config file | None required — test configuration via `#[cfg(test)]` modules |
| Quick run command | `cargo test -p server` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| AUTH-01 | register creates account with hashed password in DB | integration | `cargo test -p server test_register` | No — Wave 0 |
| AUTH-01 | hashed password is not plaintext | unit | `cargo test -p server test_hash_not_plaintext` | No — Wave 0 |
| AUTH-02 | login with correct credentials returns session token | integration | `cargo test -p server test_login_ok` | No — Wave 0 |
| AUTH-02 | login with wrong password returns error | integration | `cargo test -p server test_login_bad_password` | No — Wave 0 |
| AUTH-02 | session token is valid across multiple uses | integration | `cargo test -p server test_session_persistence` | No — Wave 0 |
| AUTH-08 | logout invalidates the session token | integration | `cargo test -p server test_logout_invalidates_session` | No — Wave 0 |
| NETW-01 | 10 concurrent connections don't block each other | integration | `cargo test -p server test_concurrent_connections` | No — Wave 0 |
| NETW-04 | protocol crate compiles with server and client-tui | build smoke | `cargo build --workspace` | No — Wave 0 |
| NETW-04 | changing a protocol type causes compile error in both crates | manual (compile test) | `cargo build --workspace` after type change | No — Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p server`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full workspace test suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `server/tests/auth_integration.rs` — covers AUTH-01, AUTH-02, AUTH-08
- [ ] `server/tests/concurrent_connections.rs` — covers NETW-01
- [ ] Workspace build smoke test — covers NETW-04
- [ ] `.sqlx/` query cache must be generated: `cargo sqlx prepare --workspace` — required before `sqlx::query!()` macros compile in CI

*(No existing test infrastructure — project is greenfield)*

---

## Sources

### Primary (HIGH confidence)
- crates.io API (verified 2026-03-23) — all version numbers in Standard Stack table
- https://rustsec.org/advisories/RUSTSEC-2025-0141 — bincode unmaintained advisory
- https://github.com/bincode-org/bincode/releases — confirmed archived August 2025
- https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0 — Axum 0.8 release notes
- https://ryhl.io/blog/actors-with-tokio/ — Actor pattern canonical reference (Alice Ryhl / Tokio team)
- https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html — Argon2id minimum parameters
- https://rustcrypto.org/key-derivation/hashing-password.html — RustCrypto argon2 crate docs
- https://docs.rs/axum/latest/axum/extract/ws/index.html — Axum WebSocket extractor

### Secondary (MEDIUM confidence)
- https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html — Cargo workspace structure
- https://github.com/jamesmunns/postcard — postcard crate, serde-compatible, stable wire format
- https://docs.rs/sqlx/latest/sqlx/ — SQLx 0.8 migrations and query macros
- https://medium.com/@mikecode/axum-websocket-468736a5e1c7 — Axum WebSocket + broadcast pattern

### Tertiary (LOW confidence — verify before use)
- Community guidance on jsonwebtoken 10.x backend feature selection — verify against https://github.com/Keats/jsonwebtoken before Phase 6

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all versions verified against crates.io on 2026-03-23; bincode replacement confirmed with official advisory
- Architecture: HIGH — workspace pattern is official Cargo documentation; actor pattern from Tokio team blog
- Pitfalls: HIGH — SQLx offline mode and Argon2 blocking are widely documented; bincode advisory is official RustSec

**Research date:** 2026-03-23
**Valid until:** 2026-06-23 (90 days — stable ecosystem; postcard/axum/sqlx are not fast-moving)
