# MUT Remastered

## What This Is

A modern Multi-User Text dungeon (MUD) built in Rust with a persistent world, D&D-flavored mechanics, and a rich Unicode TUI that runs in standard terminals (iTerm2, xterm). Players can connect via a native terminal client or a browser-based TUI (xterm.js). The web frontend also serves as a character armory and server hub.

## Core Value

Players can explore a shared persistent dungeon world together through a beautiful terminal interface — social interaction and exploration come first, combat second.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] Persistent game server that maintains world state across sessions
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
| Rust for server + native client | Performance, safety, Ratatui ecosystem | — Pending |
| Rich Unicode TUI over GPU 3D | Terminal compatibility (iTerm2/xterm) | — Pending |
| Dual client (native + web) | Player choice, accessibility | — Pending |
| Persistent world over sessions | Social/exploration focus needs continuity | — Pending |
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
*Last updated: 2026-03-23 after initialization*
