---
phase: 1
slug: server-foundation
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-23
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in (`cargo test`) + `#[tokio::test]` for async |
| **Config file** | None required — test configuration via `#[cfg(test)]` modules |
| **Quick run command** | `cargo test -p server` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p server`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 1-01-01 | 01 | 1 | AUTH-01 | integration | `cargo test -p server test_register` | ❌ W0 | ⬜ pending |
| 1-01-02 | 01 | 1 | AUTH-01 | unit | `cargo test -p server test_hash_not_plaintext` | ❌ W0 | ⬜ pending |
| 1-01-03 | 01 | 1 | AUTH-02 | integration | `cargo test -p server test_login_ok` | ❌ W0 | ⬜ pending |
| 1-01-04 | 01 | 1 | AUTH-02 | integration | `cargo test -p server test_login_bad_password` | ❌ W0 | ⬜ pending |
| 1-01-05 | 01 | 1 | AUTH-02 | integration | `cargo test -p server test_session_persistence` | ❌ W0 | ⬜ pending |
| 1-01-06 | 01 | 1 | AUTH-08 | integration | `cargo test -p server test_logout_invalidates_session` | ❌ W0 | ⬜ pending |
| 1-01-07 | 01 | 1 | NETW-01 | integration | `cargo test -p server test_concurrent_connections` | ❌ W0 | ⬜ pending |
| 1-01-08 | 01 | 1 | NETW-04 | build smoke | `cargo build --workspace` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `server/tests/auth_integration.rs` — stubs for AUTH-01, AUTH-02, AUTH-08
- [ ] `server/tests/concurrent_connections.rs` — stubs for NETW-01
- [ ] Workspace build smoke test — covers NETW-04
- [ ] `.sqlx/` query cache generation: `cargo sqlx prepare --workspace`

*Project is greenfield — no existing test infrastructure.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Changing a protocol type causes compile error in both crates | NETW-04 | Requires intentional type breakage | 1. Modify a type in `protocol/src/lib.rs` 2. Run `cargo build --workspace` 3. Verify compile errors in both `server` and `client-tui` |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
