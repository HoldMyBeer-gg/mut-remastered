---
phase: 01-server-foundation
plan: 01
subsystem: infra
tags: [rust, cargo-workspace, postcard, sqlx, sqlite, axum, argon2, tokio, tracing]

requires: []
provides:
  - Cargo workspace with resolver=2 and three member crates (protocol, server, client-tui)
  - Shared protocol crate with ClientMsg/ServerMsg/ErrorCode types and postcard codec
  - Length-prefixed (4-byte LE) binary framing for TCP protocol messages
  - SQLite database schema with accounts and sessions tables, WAL mode, foreign keys
  - Server entry point with tracing init, config from env, and sqlx migration runner
affects: [02-server-foundation, 03-server-foundation, phase-2, phase-3, phase-4, phase-5, phase-6, phase-7]

tech-stack:
  added:
    - postcard 1.1.3 (binary serialization, replaces unmaintained bincode)
    - sqlx 0.8.6 with sqlite + migrate features
    - axum 0.8.8 with ws feature
    - argon2 0.5.3 (Argon2id password hashing)
    - tracing + tracing-subscriber 0.3
    - tower-http 0.6 (CORS, compression)
    - uuid 1.x, chrono 0.4, serde_json 1.0
  patterns:
    - Cargo workspace resolver=2 with shared [workspace.dependencies]
    - Path dependencies for intra-workspace crates (protocol = { path = "../protocol" })
    - Length-prefixed binary framing: 4-byte LE u32 + postcard payload
    - sqlx migrate! macro for schema migrations at server startup
    - WAL mode + foreign_keys PRAGMA on database connection
    - EnvFilter tracing setup with fallback default

key-files:
  created:
    - Cargo.toml (workspace root with shared dependencies)
    - protocol/Cargo.toml
    - protocol/src/lib.rs
    - protocol/src/auth.rs (ClientMsg, ServerMsg, ErrorCode enums)
    - protocol/src/codec.rs (encode_message, decode_message with length prefix)
    - client-tui/Cargo.toml
    - client-tui/src/main.rs (stub: exercises codec round-trip)
    - server/Cargo.toml
    - server/src/main.rs (tokio::main, tracing, db init)
    - server/src/config.rs (ServerConfig from env)
    - server/src/db.rs (init_db with WAL, FK, migrate!)
    - server/migrations/001_accounts.sql (accounts + sessions schema)
    - .gitignore
  modified: []

key-decisions:
  - "Use postcard instead of bincode: bincode is unmaintained (RUSTSEC-2025-0141, archived Aug 2025); postcard is serde-compatible with stable wire format"
  - "UUID session tokens stored in DB rather than JWT: simpler, instantly revocable on logout (AUTH-08), JWT deferred to Phase 6 web armory"
  - "4-byte little-endian length prefix for TCP framing: standard approach, no delimiter collisions in binary data"
  - "WAL mode enabled on SQLite pool: allows concurrent reads which will be needed when multiple sessions run simultaneously"

patterns-established:
  - "Protocol pattern: all message types derive Serialize/Deserialize in protocol/ crate; consuming crates get compile errors on type change (NETW-04)"
  - "Codec pattern: encode_message prepends 4-byte LE length; decode_message takes payload slice after prefix is stripped by caller"
  - "Config pattern: ServerConfig::from_env() with unwrap_or_else defaults; no dotenv dependency needed for Phase 1"
  - "DB pattern: init_db runs PRAGMA setup then sqlx::migrate! in a single connection pool"

requirements-completed: [NETW-04, AUTH-01]

duration: 2min
completed: 2026-03-24
---

# Phase 1, Plan 01: Server Foundation Summary

**Cargo workspace with three crates (protocol, server, client-tui), postcard binary codec with 4-byte length framing, and SQLite schema with accounts/sessions tables initialized via sqlx::migrate! on server startup**

## Performance

- **Duration:** ~2 min (compile-heavy; dependency download dominated)
- **Started:** 2026-03-24T00:37:12Z
- **Completed:** 2026-03-24T00:39:34Z
- **Tasks:** 2
- **Files modified:** 13 created, 0 modified

## Accomplishments

- Cargo workspace compiles all three crates cleanly with `cargo build --workspace`
- Protocol crate enforces type safety: ClientMsg/ServerMsg/ErrorCode with serde+postcard derives; changing a type in protocol/ causes compile errors in both server/ and client-tui/
- SQLite database initializes on server startup with accounts and sessions tables, WAL mode enabled for concurrent reads, foreign key constraints enabled
- Server entry point with structured tracing output, config from environment variables, and migration runner confirmed working

## Task Commits

1. **Task 1: Create Cargo workspace and protocol crate with message types and codec** - `e0b0c98` (feat)
2. **Task 2: Create server crate with SQLite schema, database init, config, and entry point** - `29a3331` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `Cargo.toml` - Workspace root with resolver=2 and shared dependency declarations
- `protocol/src/auth.rs` - ClientMsg, ServerMsg, ErrorCode enums with serde+postcard derives
- `protocol/src/codec.rs` - encode_message (4-byte LE prefix) and decode_message functions
- `client-tui/src/main.rs` - Stub that exercises codec round-trip proving NETW-04
- `server/src/config.rs` - ServerConfig loading BIND_ADDR, DATABASE_URL, SESSION_TTL_SECS from env
- `server/src/db.rs` - init_db with WAL PRAGMA, FK PRAGMA, and sqlx::migrate! runner
- `server/src/main.rs` - tokio::main entry point with tracing, config, and database initialization
- `server/migrations/001_accounts.sql` - accounts and sessions tables with indexes
- `.gitignore` - Excludes /target, *.db files, and .env

## Decisions Made

- **postcard over bincode**: bincode is unmaintained as of RUSTSEC-2025-0141 (archived August 2025). postcard is serde-compatible, has a stable wire format spec, and 25M+ downloads.
- **UUID session tokens over JWT**: For TCP native client sessions, DB-backed UUIDs are simpler and instantly revocable on logout (satisfies AUTH-08). JWT deferred to Phase 6 when web armory needs stateless HTTP tokens.
- **4-byte LE length prefix for TCP framing**: Standard approach for binary protocols over TCP streams; avoids delimiter collision issues with binary payloads.
- **WAL mode on SQLite**: Enables concurrent reads across multiple connection futures without write-blocking reads; necessary as player count grows.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed unused `ServerMsg` import in client-tui**
- **Found during:** Task 1 (workspace build verification)
- **Issue:** Plan's client-tui/src/main.rs imported `ServerMsg` but didn't use it, generating a compiler warning
- **Fix:** Removed `ServerMsg` from the use statement — only `ClientMsg` is needed for the Ping round-trip test
- **Files modified:** client-tui/src/main.rs
- **Verification:** `cargo build --workspace` completes with zero warnings
- **Committed in:** e0b0c98 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - bug/warning)
**Impact on plan:** Minor cleanup; no scope change.

## Issues Encountered

- SQLite database URL requires `?mode=rwc` suffix to create the file on first run; running without the suffix from a directory lacking write access returns error code 14. Verified by running with full URL from /tmp. The default config in `ServerConfig::from_env()` already includes `?mode=rwc` so this is a non-issue in normal usage.

## User Setup Required

None - no external service configuration required. SQLite is bundled in sqlx's `sqlite` feature. Server creates the database file automatically on first startup.

Note: `.env` file is gitignored. For local development, create `.env` in project root with:
```
DATABASE_URL=sqlite://./mut_remastered.db?mode=rwc
BIND_ADDR=127.0.0.1:4000
RUST_LOG=server=debug
```

## Known Stubs

- `client-tui/src/main.rs`: Full TUI implementation deferred to Phase 4. Current stub only exercises the codec round-trip to prove NETW-04. This is intentional per plan design.
- `server/src/main.rs`: TCP listener accept loop not yet implemented — placeholder log message. Added in Plan 02.

## Next Phase Readiness

- Workspace structure established; all downstream plans compile against protocol/
- Database schema ready for Plan 02 (auth handlers) and Plan 03 (session actor)
- Config, tracing, and db init patterns established for use in all subsequent server plans
- No blockers for Plan 02

---

*Phase: 01-server-foundation*
*Completed: 2026-03-24*
