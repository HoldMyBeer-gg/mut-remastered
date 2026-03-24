# Phase 02: World and Movement - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-03-24
**Phase:** 02-world-and-movement
**Areas discussed:** Room data format, World tick model, Newbie area design, Interactable objects
**Mode:** --auto (all decisions auto-selected)

---

## Room Data Format

| Option | Description | Selected |
|--------|-------------|----------|
| TOML/JSON data files + SQLite state overlay | Rooms defined in text files, state mutations persisted in DB | auto |
| Pure SQL rows | All room data in database tables | |
| In-memory Rust structs only | Hardcoded world, no external data | |

**User's choice:** [auto] TOML/JSON data files + SQLite state overlay
**Notes:** Recommended default — separates authored content from runtime state, version-controllable world data

## World Tick Model

| Option | Description | Selected |
|--------|-------------|----------|
| Event-driven (command → response) | Process commands immediately, no tick loop | auto |
| Fixed-tick ECS loop | Scheduled world updates at fixed intervals | |
| Hybrid (event + periodic) | Commands immediate, scheduled tasks on timer | |

**User's choice:** [auto] Event-driven command processing
**Notes:** Phase 2 has no time-based systems. ECS can be introduced in Phase 3 for combat rounds.

## Newbie Area Design

| Option | Description | Selected |
|--------|-------------|----------|
| Freely explorable with contextual hints | Open zone with hints triggered by room entry/commands | auto |
| Forced linear tutorial path | Gated rooms requiring completion before next area | |
| Hint overlay on regular world | No special area, just help prompts everywhere | |

**User's choice:** [auto] Freely explorable with contextual hints
**Notes:** Linear paths feel restrictive in a MUD. Contextual hints preserve exploration freedom.

## Interactable Objects

| Option | Description | Selected |
|--------|-------------|----------|
| Data-driven triggers in room TOML | Triggers defined as data with conditions and effects | auto |
| Hardcoded per-room handlers | Rust functions per interactable | |
| Scripting engine (Lua/Rhai) | Embedded scripting for trigger logic | |

**User's choice:** [auto] Data-driven triggers in room TOML
**Notes:** Avoids hardcoding per-room logic, simpler than a scripting engine. All behavior from data files.

## Claude's Discretion

- Room TOML file structure and field names
- Zone/region directory organization
- Internal world data structures
- Protocol message type design
- Migration schema design

## Deferred Ideas

None
