# Roadmap: MUT Remastered

## Overview

MUT Remastered is built from the inside out: server architecture first, then world, then characters and combat, then the two client surfaces (native TUI and browser), then social systems, and finally procedural dungeon generation. Each phase delivers a coherent, playable slice that validates the one before it. The architectural spine (actor-per-session, fixed-tick ECS world loop, shared protocol crate) is established in Phase 1 and never revisited — all downstream phases compile against it. The goal is a fully playable shared-world MUD through a beautiful terminal interface by the end of Phase 7.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Server Foundation** - Tokio actor-per-session server, ECS world stub, SQLx schema, auth, and shared protocol crate (completed 2026-03-24)
- [ ] **Phase 2: World and Movement** - Room system, movement commands, persistent world state, newbie area, and lore
- [ ] **Phase 3: Character and Combat** - Character creation, D&D-flavored combat, inventory/equipment, and NPC monsters
- [ ] **Phase 4: Native TUI Client** - Ratatui split-panel TUI binary with minimap, vitals bar, and full iTerm2/xterm compatibility
- [ ] **Phase 5: Chat and Social** - Local/global chat channels, IC/OOC separation, player inspection, and character descriptions
- [ ] **Phase 6: Browser Client** - xterm.js web client with feature parity, WebSocket transport, and flow control
- [ ] **Phase 7: Procedural Dungeons** - BSP dungeon generator, hand-crafted set piece injection, connectivity guarantees, and dungeon lore

## Phase Details

### Phase 1: Server Foundation
**Goal**: A secure, correctly-architected game server that players can connect to and authenticate against
**Depends on**: Nothing (first phase)
**Requirements**: AUTH-01, AUTH-02, AUTH-08, NETW-01, NETW-04
**Success Criteria** (what must be TRUE):
  1. A player can create an account with a username and password, and the password is stored hashed (never plaintext)
  2. A player can log in and receive a session that persists across multiple commands without re-authenticating
  3. A player can log out cleanly and their session is invalidated
  4. Multiple players can connect simultaneously over TCP without one player's actions blocking another
  5. Native and web clients share a single compiled protocol crate — a message type change in the server produces a compile error in both clients
**Plans:** 3/3 plans complete
Plans:
- [x] 01-01-PLAN.md — Cargo workspace scaffold, protocol crate with message types and codec, SQLite schema and server entry point
- [x] 01-02-PLAN.md — Auth module (Argon2id hashing, session management), TCP listener, and connection actor
- [x] 01-03-PLAN.md — Integration tests validating all Phase 1 requirements end-to-end

### Phase 2: World and Movement
**Goal**: Players can explore a persistent hand-crafted world and discover its lore
**Depends on**: Phase 1
**Requirements**: WRLD-01, WRLD-02, WRLD-03, WRLD-04, WRLD-05, WRLD-06
**Success Criteria** (what must be TRUE):
  1. A player can type a cardinal direction (n, s, e, w, u, d) and move to an adjacent room, seeing a full room description and exit list on arrival
  2. World state — room contents, player positions — survives a server restart; players log back in where they left off
  3. A new player entering the newbie area receives guided prompts that explain movement and basic commands before entering the wider world
  4. Rooms contain readable lore text that rewards players who type "look" or examine environmental details
  5. A player action that should change world state (e.g., pulling a lever, clearing a blockage) is reflected for all players in that area
**Plans**: TBD

### Phase 3: Character and Combat
**Goal**: Players can create characters, fight monsters, collect gear, and die without losing progress
**Depends on**: Phase 2
**Requirements**: AUTH-03, AUTH-04, AUTH-05, AUTH-06, AUTH-07, CHAR-01, CHAR-02, CHAR-03, CHAR-04, CHAR-05, CMBT-01, CMBT-02, CMBT-03, CMBT-04, CMBT-05, CMBT-06
**Success Criteria** (what must be TRUE):
  1. A player can create a new character by choosing race, class, allocating ability scores, and selecting gender — character appears in the world immediately after
  2. Combat log shows dice roll results in plain language ("rolled 14 + 2 STR vs AC 12 — HIT! 7 damage") and resolves in ~2-second rounds
  3. HP, mana, and stamina are always visible in the game interface without the player needing to type a command
  4. A player can pick up items, drop them, and equip gear to named body slots — equipped gear changes their displayed stats
  5. When a player dies, they respawn at their bind point with gear intact and an XP debt rather than losing everything
**Plans**: TBD
**UI hint**: yes

### Phase 4: Native TUI Client
**Goal**: The native terminal client is a polished, split-panel experience that works correctly on both iTerm2 and xterm
**Depends on**: Phase 3
**Requirements**: TUI-01, TUI-02, TUI-03, TUI-04
**Success Criteria** (what must be TRUE):
  1. The native client displays a split-panel layout with a room description pane, vitals bar, chat pane, and input line — all visible simultaneously without scrolling
  2. The client renders correctly (no box-drawing corruption, no color bleed) in both iTerm2 on macOS and xterm on Linux
  3. A minimap or compass overlay shows explored rooms with fog of war and the player's current position
  4. Resizing the terminal window at any time reflows all panels without crashes or visual artifacts
**Plans**: TBD
**UI hint**: yes

### Phase 5: Chat and Social
**Goal**: Players can communicate in-character and out-of-character, and inspect each other's characters
**Depends on**: Phase 4
**Requirements**: SOCL-01, SOCL-02, SOCL-03, SOCL-04, SOCL-05, SOCL-06
**Success Criteria** (what must be TRUE):
  1. A player can "say" something and only players in the same room see it; "gossip" is visible to all online players
  2. IC and OOC channels are visually distinct — say/emote are clearly marked as in-character, gossip as out-of-character
  3. A player can toggle a channel off and stop receiving its messages, then toggle it back on
  4. Typing "look at [player]" shows that player's equipped gear, visible stats, and biography
  5. A player can set a visible character description (e.g., "a weathered dwarf with a scarred brow") that other players see when they look at them
**Plans**: TBD
**UI hint**: yes

### Phase 6: Browser Client
**Goal**: Players can connect and play from any modern browser with full feature parity to the native client
**Depends on**: Phase 5
**Requirements**: NETW-02, NETW-03
**Success Criteria** (what must be TRUE):
  1. A player can open a URL in a browser (no install required) and connect to the game server, seeing the same split-panel TUI as the native client
  2. All game features available in the native client — movement, combat, chat, character inspection — work identically in the browser client
  3. If a player's browser tab loses connection briefly (up to 60 seconds), they can reconnect without being fully logged out
**Plans**: TBD
**UI hint**: yes

### Phase 7: Procedural Dungeons
**Goal**: Players can descend into algorithmically generated dungeons with hand-crafted boss rooms and guaranteed navigability
**Depends on**: Phase 6
**Requirements**: DUNG-01, DUNG-02, DUNG-03, DUNG-04
**Success Criteria** (what must be TRUE):
  1. Each dungeon floor is unique, generated by algorithm, with rooms and corridors laid out differently each time
  2. Every room in a generated dungeon floor is reachable — no isolated dead-end clusters or disconnected rooms
  3. Boss rooms and hand-crafted set pieces appear at predictable anchor points within procedural floors, offering premium encounter design within generated layouts
  4. Generated rooms have flavor text and lore that fit the dungeon's theme, not generic filler
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Server Foundation | 3/3 | Complete   | 2026-03-24 |
| 2. World and Movement | 0/? | Not started | - |
| 3. Character and Combat | 0/? | Not started | - |
| 4. Native TUI Client | 0/? | Not started | - |
| 5. Chat and Social | 0/? | Not started | - |
| 6. Browser Client | 0/? | Not started | - |
| 7. Procedural Dungeons | 0/? | Not started | - |
