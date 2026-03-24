# MUT Remastered

## What This Is

A modern Multi-User Text dungeon (MUD) built in Rust with a persistent world, D&D-flavored mechanics, and a rich Unicode TUI that runs in standard terminals (iTerm2, xterm). Players can connect via a native terminal client or a browser-based TUI (xterm.js). The web frontend also serves as a character armory and server hub.

## Core Value

Players can explore a shared persistent dungeon world together through a beautiful terminal interface — social interaction and exploration come first, combat second.

## Requirements

### Validated

- [x] Persistent game server that maintains world state across sessions — Validated in Phase 01: Server Foundation (SQLite-backed accounts/sessions, server starts and applies migrations)
- [x] Secure account creation with Argon2id password hashing — Validated in Phase 01: Server Foundation (AUTH-01)
- [x] Session-based authentication with login/logout — Validated in Phase 01: Server Foundation (AUTH-02, AUTH-08)
- [x] Concurrent TCP connections without blocking — Validated in Phase 01: Server Foundation (NETW-01, 10 concurrent connections tested)
- [x] Shared protocol crate with compile-time type safety — Validated in Phase 01: Server Foundation (NETW-04)
- [x] Rooms with rich text descriptions and visible exits — Validated in Phase 02: World and Movement (WRLD-01)
- [x] Cardinal direction movement between rooms — Validated in Phase 02: World and Movement (WRLD-02)
- [x] World state persists across server restarts — Validated in Phase 02: World and Movement (WRLD-03, player positions + world state in SQLite)
- [x] Newbie tutorial area with guided prompts — Validated in Phase 02: World and Movement (WRLD-04, 7-room Warden's Academy)
- [x] Embedded lore rewarding exploration — Validated in Phase 02: World and Movement (WRLD-05)
- [x] Persistent world-state consequences from player actions — Validated in Phase 02: World and Movement (WRLD-06, data-driven trigger system)

### Active

- [ ] Rich Unicode TUI client (Ratatui) compatible with iTerm2 and xterm
- [ ] Browser-based TUI client (xterm.js) with same capabilities as native client
- [ ] Web app launcher/hub for account management and server connection
- [ ] D&D-flavored character system (ability scores, classes, levels, streamlined rules)
- [ ] Dice-based skill checks and combat (attack rolls, AC, HP)
- [ ] Procedurally generated dungeons with algorithmic room/corridor/monster placement
- [ ] Hand-crafted set pieces and boss rooms within procedural dungeons
- [ ] Local chat for players in the same room/area
- [ ] Global chat for server-wide communication
- [ ] Character inspection panel in TUI (gear, stats, abilities)
- [ ] Web-based character armory (public profiles, gear visualization, stats)
- [ ] Player-to-player inspection (view other players' characters)
- [ ] Social exploration-focused gameplay (room descriptions, lore, discoverable areas)
- [ ] 3D models created with blend-ai for web armory character/gear visualization

### Out of Scope

- GPU-accelerated terminal rendering — not compatible with xterm
- Real-time voice chat — text-first social experience
- Mobile native app — web client covers mobile access
- PvP arena system — social/exploration focus for v1
- Player housing — defer to future milestone
- Trading/auction house — defer to future milestone

## Context

- Terminal compatibility is a hard constraint: must work in iTerm2 (macOS) and xterm (Linux)
- The project is called "MUT Remastered" suggesting a revival/reimagining of a previous MUT project
- Social interaction and exploration are prioritized over combat min-maxing
- D&D rules are inspiration, not strict implementation — streamlined for fun over simulation
- blend-ai is the tool for generating any 3D models needed (character/gear visualization on web armory)

## Constraints

- **Terminal compatibility**: Must render correctly in iTerm2 and xterm using Unicode/truecolor — no terminal-specific protocols
- **Language**: Rust for server and native TUI client
- **Web client**: Browser-based TUI via xterm.js, web app for armory/hub
- **3D assets**: Use blend-ai for any model generation
- **Rules system**: D&D-flavored but simplified — not a strict SRD implementation

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust for server + native client | Performance, safety, Ratatui ecosystem | Validated (Phase 01) |
| Rich Unicode TUI over GPU 3D | Terminal compatibility (iTerm2/xterm) | — Pending |
| Dual client (native + web) | Player choice, accessibility | — Pending |
| Persistent world over sessions | Social/exploration focus needs continuity | Validated (Phase 02) |
| Procedural + hand-crafted dungeons | Replayability with quality set pieces | — Pending |
| D&D-flavored over strict SRD | Fun and accessibility over simulation accuracy | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd:transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-03-24 after Phase 02 completion — world and movement verified with 19 passing tests (8 auth + 11 world)*
