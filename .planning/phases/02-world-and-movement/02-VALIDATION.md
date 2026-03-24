---
phase: 02
slug: world-and-movement
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-24
---

# Phase 02 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml (workspace) |
| **Quick run command** | `cargo test -p server --lib` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p server --lib`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 02-01-01 | 01 | 1 | WRLD-01 | unit | `cargo test -p server world` | ❌ W0 | ⬜ pending |
| 02-01-02 | 01 | 1 | WRLD-02 | unit | `cargo test -p server world::movement` | ❌ W0 | ⬜ pending |
| 02-02-01 | 02 | 2 | WRLD-03 | integration | `cargo test -p server persistence` | ❌ W0 | ⬜ pending |
| 02-02-02 | 02 | 2 | WRLD-06 | integration | `cargo test -p server world_state` | ❌ W0 | ⬜ pending |
| 02-03-01 | 03 | 3 | WRLD-04 | integration | `cargo test -p server tutorial` | ❌ W0 | ⬜ pending |
| 02-03-02 | 03 | 3 | WRLD-05 | integration | `cargo test -p server lore` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `server/tests/world_integration.rs` — stubs for WRLD-01, WRLD-02 movement tests
- [ ] `server/tests/persistence_integration.rs` — stubs for WRLD-03 world state persistence
- [ ] `server/tests/helpers/mod.rs` — update TestServer to include World state

*Existing test infrastructure from Phase 1 covers framework setup.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Newbie area guided prompts feel natural | WRLD-04 | Subjective UX quality | Connect as new player, walk through newbie zone, verify hints appear at appropriate times |
| Lore text rewards exploration | WRLD-05 | Content quality assessment | Visit rooms, use "look" and "examine", verify descriptions are engaging |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
