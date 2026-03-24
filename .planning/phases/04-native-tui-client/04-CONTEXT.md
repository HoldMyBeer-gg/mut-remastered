# Phase 04: Native TUI Client - Context

**Gathered:** 2026-03-24
**Status:** Ready for planning

<domain>
## Phase Boundary

The native terminal client is a polished Ratatui-based TUI that connects to the MUT server, handles login/character selection, and presents a split-panel game interface. This phase delivers TUI-01 through TUI-04.

**Not in scope:** Chat channels (Phase 5), browser client (Phase 6), procedural dungeons (Phase 7).
</domain>

<decisions>
## Implementation Decisions

### TUI Layout (D-TUI-01)
- The main game screen has 4 regions: room description pane (top-left), minimap/compass (top-right), game log/chat (middle), vitals bar (bottom bar), and input line (very bottom).
- The layout uses Ratatui constraint-based layout: top section splits 70/30, middle takes remaining, bottom is fixed 2-3 rows.

### Connection Architecture (D-TUI-02)  
- The client uses tokio-tungstenite... NO — the server uses raw TCP with length-prefixed frames.
- The client uses tokio::net::TcpStream with the same framing as the test helpers.
- A background task reads server messages and sends them to the TUI via tokio::sync::mpsc.
- User input goes through a separate mpsc channel to a send task.

### State Machine (D-TUI-03)
- Client states: Connecting → Login → CharacterSelect → InGame
- Each state has its own UI rendering function
- The InGame state manages the game panels

### Minimap (D-TUI-04)
- Simple compass-style display showing known exits from current room
- Track explored rooms in a HashMap<String, ExploredRoom> for fog of war
- Display as a grid of Unicode box characters centered on current position

### Claude's Discretion
- Exact color palette and styling
- Widget implementations
- Minimap rendering algorithm details
- Input parsing and command handling
</decisions>

<canonical_refs>
## Canonical References
- `client-tui/src/main.rs` — Current stub to replace
- `client-tui/Cargo.toml` — Dependencies to add
- `protocol/src/` — All message types the client must handle
- `server/tests/helpers/mod.rs` — TCP framing reference (same pattern for client)
</canonical_refs>
