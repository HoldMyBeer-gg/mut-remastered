---
phase: 01-server-foundation
plan: 02
subsystem: auth+net
tags: [rust, argon2id, tcp, actor-per-connection, sqlx, sqlite, session-management]

requires:
  - 01-01 (protocol crate with ClientMsg/ServerMsg, codec, DB schema, server entry point)
provides:
  - Argon2id password hashing (hash_password, verify_password) via argon2 crate
  - Session CRUD (create/validate/delete_session, register/lookup_account) against SQLite sessions table
  - TCP accept loop spawning one independent Tokio task per connection (NETW-01)
  - ConnectionActor: per-connection state machine handling Register/Login/Logout/Ping
  - AUTH-08: session deleted on explicit logout AND on graceful disconnect
affects:
  - 01-03-server-foundation (next plan builds on actor pattern)
  - All future phases connect through this auth layer

tech-stack:
  added:
    - argon2 0.5.3 with rand feature (Argon2id + OsRng salt generation)
    - rand_core 0.6 with getrandom feature (OsRng support)
  patterns:
    - Argon2id hashing always via tokio::task::spawn_blocking (CPU-intensive, not on async thread)
    - PHC string format for stored password hashes
    - UUID v4 tokens for sessions (instantly revocable, no JWT complexity in TCP layer)
    - Actor-per-connection: ConnectionActor owns OwnedReadHalf + OwnedWriteHalf + AppState clone
    - AppState: Clone-able struct with SqlitePool + session_ttl_secs shared across connections
    - Frame reading: 4-byte LE length prefix, max 64 KiB, EOF detection returns None

key-files:
  created:
    - server/src/auth/mod.rs
    - server/src/auth/hash.rs (hash_password, verify_password using Argon2id)
    - server/src/auth/session.rs (create_session, validate_session, delete_session, register_account, lookup_account)
    - server/src/net/mod.rs
    - server/src/net/listener.rs (run_listener, AppState, handle_connection)
    - server/src/session/mod.rs
    - server/src/session/actor.rs (ConnectionActor with run, handle_message, send, cleanup, read_frame)
  modified:
    - server/Cargo.toml (added argon2 rand feature, rand_core with getrandom)
    - server/src/main.rs (added mod auth/net/session, AppState init, run_listener call)

key-decisions:
  - "Argon2id hashing on spawn_blocking: Argon2 is memory-hard and CPU-intensive; calling it directly in an async task would block the Tokio thread pool and degrade all concurrent connections"
  - "argon2 rand feature + rand_core/getrandom explicit dep: OsRng requires getrandom feature in rand_core; argon2's rand feature enables password-hash/rand_core but not getrandom; explicit rand_core dep needed"
  - "validate_session defined but unused in Plan 02: function is exported for future session validation (e.g., reconnect with existing token). Dead code warning intentional — used in future plans"
  - "delete_session in both Logout handler and cleanup(): logout frees the slot immediately; cleanup on connection drop handles crash/network failure cases — AUTH-08 requires both paths"

metrics:
  duration: ~3 min
  completed: 2026-03-24
  tasks: 2
  files_created: 7
  files_modified: 2
---

# Phase 1, Plan 02: Auth Module and TCP Listener Summary

**Argon2id password hashing, DB-backed session management, TCP accept loop with one Tokio task per connection, and ConnectionActor that handles Register/Login/Logout/Ping messages with full session lifecycle**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-03-24T00:41:38Z
- **Completed:** 2026-03-24T00:44:20Z
- **Tasks:** 2
- **Files created:** 7, **Files modified:** 2

## Accomplishments

- Auth module compiles with Argon2id (PHC string format, random salt per hash)
- Session CRUD works against SQLite sessions table using runtime queries (no compile-time DB requirement)
- TCP listener spawns one independent Tokio task per accepted connection — NETW-01 satisfied
- ConnectionActor processes the full auth flow: Register → hash password → store → LoginOk; Login → lookup → verify → create session → LoginOk; Logout → delete session → LogoutOk; Ping → Pong
- Argon2 operations run via `spawn_blocking` so they never block the Tokio async thread pool
- Session deleted on both explicit Logout AND on connection drop (AUTH-08 both paths covered)
- `cargo build --workspace` completes with zero errors, one expected dead_code warning (validate_session unused until Plan 03+)

## Task Commits

1. **Task 1: Auth module (Argon2id hashing + session CRUD)** - `3df33a7` (feat)
2. **Task 2: TCP listener and connection actor** - `59207c2` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `server/src/auth/hash.rs` - hash_password (Argon2id + OsRng salt) and verify_password
- `server/src/auth/session.rs` - create/validate/delete_session, register/lookup_account with runtime sqlx queries
- `server/src/net/listener.rs` - run_listener accept loop, AppState struct, handle_connection spawner
- `server/src/session/actor.rs` - ConnectionActor: read_frame (4-byte LE prefix), handle_message dispatch, send, cleanup
- `server/Cargo.toml` - argon2 rand feature + rand_core getrandom feature added
- `server/src/main.rs` - mod auth/net/session, AppState construction, run_listener call, placeholder removed

## Decisions Made

- **Argon2id on spawn_blocking**: Argon2 is memory-hard (19 MiB RAM per hash) and CPU-intensive. Running it in async context would stall the thread and degrade all other connections. `tokio::task::spawn_blocking` isolates it to the blocking thread pool.
- **rand_core getrandom explicit dep**: The argon2 `rand` feature enables `password-hash/rand_core` but does not propagate the `getrandom` feature flag needed for `OsRng`. Adding `rand_core = { version = "0.6", features = ["getrandom"] }` explicitly resolves this.
- **validate_session exported but unused**: Defined for completeness of the session API. Future plans (reconnect with token, HTTP API sessions) will use it. Dead-code warning is expected and acceptable.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed argon2 OsRng compilation error**
- **Found during:** Task 1 (first cargo build)
- **Issue:** `argon2 = "0.5"` does not transitively enable `getrandom` feature on `rand_core`, so `OsRng` was gated out at compile time
- **Fix:** Added `rand` feature to argon2 dep; added explicit `rand_core = { version = "0.6", features = ["getrandom"] }` dependency
- **Files modified:** server/Cargo.toml
- **Committed in:** 3df33a7 (Task 1 commit)

**2. [Rule 1 - Bug] Fixed read_exact Ok pattern match**
- **Found during:** Task 2 (cargo build after actor.rs created)
- **Issue:** tokio's `read_exact` returns `Result<usize>` not `Result<()>`; pattern `Ok(())` did not compile
- **Fix:** Changed `Ok(())` to `Ok(_)` to accept the byte count return value
- **Files modified:** server/src/session/actor.rs
- **Committed in:** 59207c2 (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (Rule 1 - bug)
**Impact on plan:** Minor compile fixes; no behavior or scope change.

## Known Stubs

- `auth::session::validate_session`: Function is complete but unused in this plan. Will be called when session reconnection is implemented in a future plan.
- `client-tui/src/main.rs`: Full TUI implementation still deferred to Phase 4 (carried over from Plan 01).

## Next Phase Readiness

- Auth layer is production-quality: Argon2id hashing, UUID session tokens, DB-backed expiry
- TCP listener and actor pattern established for Plan 03 (game world actor integration)
- AppState is Clone and extensible — add fields as new subsystems come online
- No blockers for Plan 03

---

*Phase: 01-server-foundation*
*Completed: 2026-03-24*

## Self-Check: PASSED

- server/src/auth/hash.rs: FOUND
- server/src/auth/session.rs: FOUND
- server/src/net/listener.rs: FOUND
- server/src/session/actor.rs: FOUND
- Commit 3df33a7: FOUND
- Commit 59207c2: FOUND
