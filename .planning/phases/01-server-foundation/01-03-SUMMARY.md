---
phase: 01-server-foundation
plan: 03
subsystem: testing
tags: [rust, integration-tests, tokio, sqlx, sqlite, argon2id, tcp, concurrency]

requires:
  - 01-01 (protocol crate: ClientMsg/ServerMsg/ErrorCode, codec, DB schema)
  - 01-02 (auth module, TCP listener, ConnectionActor)
provides:
  - Integration test suite proving all Phase 1 requirements end-to-end over TCP
  - server/src/lib.rs re-exporting server modules for test crate access
  - TestServer helper (random port, isolated in-memory SQLite per test)
  - TestClient helper (typed send/recv over TCP with length-prefix framing)
  - AUTH-01: test_register_creates_account + test_hash_is_not_plaintext
  - AUTH-02: test_login_with_correct_credentials + test_login_with_wrong_password
  - AUTH-08: test_logout_invalidates_session
  - NETW-01: test_10_concurrent_connections
  - NETW-04: cargo test --workspace proves protocol compiles into all three crates
affects:
  - Phase 2+ (regression gate — any auth/networking breakage will fail these tests)

tech-stack:
  added:
    - tokio dev-dependency with full features (test harness async runtime)
  patterns:
    - Rust integration test pattern: lib.rs re-exports pub modules; tests/ crate accesses via crate name
    - In-memory SQLite per-test isolation: sqlite:file:testdb_UUID?mode=memory&cache=shared
    - Port-0 probe bind: bind, record addr, drop, let server rebind (avoids passing TcpListener through API)
    - JoinSet for concurrent task orchestration in tests

key-files:
  created:
    - server/src/lib.rs (re-exports auth/config/db/net/session for integration test access)
    - server/tests/helpers/mod.rs (TestServer + TestClient test utilities)
    - server/tests/auth_integration.rs (7 tests covering AUTH-01/02/08 and ping-pong)
    - server/tests/concurrent_connections.rs (NETW-01 concurrency test with 10 tasks)
  modified:
    - server/src/main.rs (updated imports to use server:: path from lib.rs)
    - server/Cargo.toml (added tokio dev-dependency for test async runtime)

key-decisions:
  - "In-memory SQLite per test: each TestServer starts with a unique named in-memory DB (sqlite:file:testdb_UUID?mode=memory&cache=shared) — prevents state bleed between concurrently running tests without temp file cleanup"
  - "lib.rs + main.rs split: Rust requires a lib target for integration tests to access crate internals; main.rs updated to use server:: paths from lib.rs instead of local mod declarations"
  - "Port-0 probe bind pattern: bind TcpListener to 0:0 to get free port, drop it, then call run_listener with the address string — avoids changing run_listener's signature for test-only concerns"

metrics:
  duration: ~2 min
  completed: 2026-03-24
  tasks: 2
  files_created: 4
  files_modified: 2
---

# Phase 1, Plan 03: Integration Tests Summary

**End-to-end TCP integration test suite with 8 passing tests validating all Phase 1 requirements: register, login, bad-password rejection, session logout, Argon2id hash format, ping-pong, and 10 concurrent connections**

## Performance

- **Duration:** ~2 min
- **Started:** 2026-03-24T00:46:38Z
- **Completed:** 2026-03-24T00:48:27Z
- **Tasks:** 2
- **Files created:** 4, **Files modified:** 2

## Accomplishments

- `cargo test --workspace` exits 0 with 8 tests passing across the full workspace
- server/src/lib.rs created: exposes pub modules for integration test crate access (standard Rust pattern)
- TestServer: starts real server on random OS-assigned port with isolated in-memory SQLite database per test; no shared state between test runs
- TestClient: typed send/recv helpers using the same 4-byte LE length-prefix protocol as production code
- AUTH-01 proven: registration creates account with non-empty account_id; Argon2id PHC format confirmed
- AUTH-02 proven: correct credentials return session token; wrong password returns InvalidCredentials
- AUTH-08 proven: logout returns LogoutOk (server deletes session from DB on explicit logout)
- NETW-01 proven: 10 concurrent Tokio tasks each registering and logging in independently — all 10 complete successfully
- NETW-04 proven: `cargo test --workspace` compiles protocol crate into server and client-tui — compile-time type safety guaranteed

## Task Commits

1. **Task 1: lib.rs, test helpers, and auth integration tests** - `8e1588e` (feat)
2. **Task 2: concurrent connections integration test** - `89a7f33` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `server/src/lib.rs` - Re-exports pub mod auth/config/db/net/session for integration test crate access
- `server/src/main.rs` - Updated to use server:: path imports from lib.rs instead of local mod declarations
- `server/Cargo.toml` - Added tokio as dev-dependency with full features
- `server/tests/helpers/mod.rs` - TestServer (random port + isolated in-memory DB) and TestClient (typed send/recv)
- `server/tests/auth_integration.rs` - 7 integration tests covering auth flow and ping-pong
- `server/tests/concurrent_connections.rs` - 1 concurrency test spawning 10 simultaneous TCP connections

## Decisions Made

- **In-memory SQLite per test**: `sqlite:file:testdb_UUID?mode=memory&cache=shared` gives each TestServer a completely isolated database. Tests can run in parallel without state bleed, no temp file cleanup required.
- **lib.rs + main.rs split**: Rust integration tests (in `tests/`) are compiled as separate crates and cannot access `mod` items from `main.rs` (binary-only). Adding `lib.rs` with `pub mod` declarations gives the test crate full access via `server::auth::hash::hash_password`, etc. `main.rs` updated to use `server::` paths.
- **Port-0 probe bind pattern**: Bind a TcpListener to `127.0.0.1:0` to discover a free port, record the local address, drop the listener, then pass the address string to `run_listener`. Avoids changing `run_listener`'s signature for test-only concerns. Small race window is acceptable for local tests.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing functionality] Added lib.rs before writing tests**
- **Found during:** Task 1 pre-implementation read
- **Issue:** The plan noted lib.rs was needed but the existing server crate had no lib target. Without it, integration tests cannot reference `server::auth::hash::hash_password` or `server::db::init_db`.
- **Fix:** Created `server/src/lib.rs` with five `pub mod` declarations and updated `server/src/main.rs` to import via `server::` paths instead of local `mod`.
- **Files modified:** server/src/lib.rs (new), server/src/main.rs
- **Committed in:** 8e1588e (Task 1 commit)

---

**Total deviations:** 1 auto-implemented (Rule 2 - required for tests to compile)
**Impact on plan:** The plan explicitly anticipated this deviation and described exactly this approach.

## Requirements Completed

- AUTH-01: Registration stores hashed (Argon2id PHC) password; account_id returned
- AUTH-02: Correct credentials grant session token; wrong password returns InvalidCredentials
- AUTH-08: Explicit logout returns LogoutOk; session deleted from DB
- NETW-01: 10 concurrent TCP connections register and login without blocking
- NETW-04: `cargo test --workspace` proves protocol crate compiles into all member crates

## Known Stubs

None — all tests are fully wired with real server instances and real database operations.

The `client-tui/src/main.rs` stub (carried from Plan 01) is outside this plan's scope.

## Phase Gate: PASSED

All Phase 1 success criteria are validated by automated tests. Phase 01-server-foundation is complete.

---

*Phase: 01-server-foundation*
*Completed: 2026-03-24*
