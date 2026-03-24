# Requirements: MUT Remastered

**Defined:** 2026-03-23
**Core Value:** Players can explore a shared persistent dungeon world together through a beautiful terminal interface — social interaction and exploration come first, combat second.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Authentication

- [ ] **AUTH-01**: User can create an account with username and hashed password
- [ ] **AUTH-02**: User can log in and receive a persistent session
- [ ] **AUTH-03**: User can create multiple characters per account
- [ ] **AUTH-04**: User can select character race from available races
- [ ] **AUTH-05**: User can select character class from 4-6 available classes
- [ ] **AUTH-06**: User can allocate 6 ability scores (STR/DEX/CON/INT/WIS/CHA)
- [ ] **AUTH-07**: User can select character gender (male/female/non-binary)
- [ ] **AUTH-08**: User can log out cleanly with automatic save

### World

- [ ] **WRLD-01**: Rooms have rich text descriptions and visible exits
- [ ] **WRLD-02**: User can move between rooms using cardinal directions (n/s/e/w/u/d) and aliases
- [ ] **WRLD-03**: World state persists across server restarts and player disconnects
- [ ] **WRLD-04**: A newbie/tutorial area guides first-time players through commands in a safe zone
- [ ] **WRLD-05**: Rooms contain embedded lore that rewards exploration
- [ ] **WRLD-06**: Player actions have persistent consequences that affect the world state (RPG-style reactivity)

### Combat

- [ ] **CMBT-01**: Combat uses D&D-flavored dice mechanics with visible roll results (e.g., "rolled 17 + 3 STR vs AC 14 — HIT!")
- [ ] **CMBT-02**: Combat is round-based (~2s rounds) with queued commands
- [ ] **CMBT-03**: NPC monsters spawn from tables with respawn timers
- [ ] **CMBT-04**: NPC monsters have basic AI (aggro, patrol)
- [ ] **CMBT-05**: Defeated monsters drop gear and gold from loot tables
- [ ] **CMBT-06**: Death is soft — player respawns at bind point with XP debt, gear intact

### Character

- [ ] **CHAR-01**: HP/mana/stamina are always visible in the TUI status bar
- [ ] **CHAR-02**: User can manage inventory (view, drop, pick up items)
- [ ] **CHAR-03**: User can equip/unequip gear to body slots (head, neck, body, arms, hands, legs, feet, ring x2, weapon, offhand)
- [ ] **CHAR-04**: Equipment affects character stats and combat effectiveness
- [ ] **CHAR-05**: User can write a 500-character biography/backstory for their character

### Social

- [ ] **SOCL-01**: User can use local chat commands (say, emote, whisper) visible to players in the same room
- [ ] **SOCL-02**: User can use global gossip channel visible to all online players
- [ ] **SOCL-03**: Chat channels have IC/OOC separation (say is IC, gossip is OOC)
- [ ] **SOCL-04**: User can toggle individual chat channels on/off
- [ ] **SOCL-05**: User can inspect other players ("look at [player]") to see their gear, stats, and bio
- [ ] **SOCL-06**: User can set a visible character description seen by other players

### TUI

- [ ] **TUI-01**: Native TUI client renders a split-panel layout (map pane, room description, chat, vitals bar) using Ratatui
- [ ] **TUI-02**: TUI works correctly in iTerm2 (macOS) and xterm (Linux) with Unicode and truecolor
- [ ] **TUI-03**: A 2D minimap/compass overlay shows explored rooms with fog of war and current position
- [ ] **TUI-04**: TUI handles terminal resize gracefully

### Dungeons

- [ ] **DUNG-01**: Procedurally generated dungeons with algorithmic room/corridor placement
- [ ] **DUNG-02**: Hand-crafted boss rooms and set pieces injected at anchor points in procedural layouts
- [ ] **DUNG-03**: Generated dungeons have verified connectivity (all rooms reachable)
- [ ] **DUNG-04**: Dungeon rooms include templated flavor text and lore

### Networking

- [ ] **NETW-01**: Game server handles multiple concurrent player connections via TCP
- [ ] **NETW-02**: Game server handles WebSocket connections for browser clients
- [ ] **NETW-03**: Browser-based TUI client via xterm.js has feature parity with native client
- [ ] **NETW-04**: Shared protocol crate ensures message consistency between native and web clients

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Web Armory

- **ARMR-01**: Public web page displays character profile (gear, stats, class, level)
- **ARMR-02**: 3D gear visualization using blend-ai generated models and Threlte
- **ARMR-03**: Shareable character profile URLs

### Help System

- **HELP-01**: Searchable help topics per command
- **HELP-02**: Context-sensitive help suggestions for new players

### Social Extended

- **SEXT-01**: Consensual PvP dueling arena (opt-in only)
- **SEXT-02**: Player-to-player direct trade via give command
- **SEXT-03**: Character achievement/accomplishment tracking

### Quality of Life

- **QLTY-01**: Command aliases and shortcuts configurable per player
- **QLTY-02**: Session reconnect — resume after disconnect without re-login

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Permadeath | Contradicts social/exploration focus; alienates casual players |
| Real-time combat | Terrible in text; punishes slow typists; conflicts with social interaction |
| Strict 5e SRD rules | License constraints; simulation depth conflicts with MUD pacing |
| PvP / ganking | Destroys social atmosphere; toxic dynamics for v1 |
| Player housing | Enormous scope; defer to future milestone |
| Trading / auction house | Requires economy balancing, anti-duping — scope multiplier |
| Crafting system | Complex balance problem; undermines loot satisfaction |
| Discord integration | Fragments in-game community |
| LLM-powered NPC dialogue | Unpredictable output; breaks lore; operational cost |
| GPU-accelerated rendering | Incompatible with xterm constraint |
| Mobile native app | Web TUI client covers mobile access |
| Audio / soundpacks | Out of scope for terminal-first design |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| AUTH-01 | Phase 1 | Pending |
| AUTH-02 | Phase 1 | Pending |
| AUTH-08 | Phase 1 | Pending |
| NETW-01 | Phase 1 | Pending |
| NETW-04 | Phase 1 | Pending |
| WRLD-01 | Phase 2 | Pending |
| WRLD-02 | Phase 2 | Pending |
| WRLD-03 | Phase 2 | Pending |
| WRLD-04 | Phase 2 | Pending |
| WRLD-05 | Phase 2 | Pending |
| WRLD-06 | Phase 2 | Pending |
| AUTH-03 | Phase 3 | Pending |
| AUTH-04 | Phase 3 | Pending |
| AUTH-05 | Phase 3 | Pending |
| AUTH-06 | Phase 3 | Pending |
| AUTH-07 | Phase 3 | Pending |
| CHAR-01 | Phase 3 | Pending |
| CHAR-02 | Phase 3 | Pending |
| CHAR-03 | Phase 3 | Pending |
| CHAR-04 | Phase 3 | Pending |
| CHAR-05 | Phase 3 | Pending |
| CMBT-01 | Phase 3 | Pending |
| CMBT-02 | Phase 3 | Pending |
| CMBT-03 | Phase 3 | Pending |
| CMBT-04 | Phase 3 | Pending |
| CMBT-05 | Phase 3 | Pending |
| CMBT-06 | Phase 3 | Pending |
| TUI-01 | Phase 4 | Pending |
| TUI-02 | Phase 4 | Pending |
| TUI-03 | Phase 4 | Pending |
| TUI-04 | Phase 4 | Pending |
| SOCL-01 | Phase 5 | Pending |
| SOCL-02 | Phase 5 | Pending |
| SOCL-03 | Phase 5 | Pending |
| SOCL-04 | Phase 5 | Pending |
| SOCL-05 | Phase 5 | Pending |
| SOCL-06 | Phase 5 | Pending |
| NETW-02 | Phase 6 | Pending |
| NETW-03 | Phase 6 | Pending |
| DUNG-01 | Phase 7 | Pending |
| DUNG-02 | Phase 7 | Pending |
| DUNG-03 | Phase 7 | Pending |
| DUNG-04 | Phase 7 | Pending |

**Coverage:**
- v1 requirements: 33 total
- Mapped to phases: 33
- Unmapped: 0

---
*Requirements defined: 2026-03-23*
*Last updated: 2026-03-23 after roadmap creation*
