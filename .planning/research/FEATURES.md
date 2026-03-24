# Feature Research

**Domain:** Multi-User Text Dungeon (MUD) — social/exploration-first, D&D-flavored, Unicode TUI
**Researched:** 2026-03-23
**Confidence:** MEDIUM — MUD genre is well-documented but modern comparables are sparse; WoW Armory and roguelike design are HIGH confidence reference points

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features MUD players assume exist. Missing these = product feels broken or unfinished.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Persistent world state | Player progress must survive disconnects/reboots — loss of progress is a deal-breaker | MEDIUM | Decide what persists vs resets; character progress always persists, dungeon layout can reset |
| Character creation (race, class, stats) | Every MUD since DikuMUD has this; it's genre bedrock | MEDIUM | 6 ability scores (STR/DEX/CON/INT/WIS/CHA) is the D&D-standard baseline; 4-6 classes for v1 |
| Room descriptions with exits | MUD navigation primitive — players expect "look" to return rich text + cardinal exits | LOW | Must be present on every room; no "empty room" filler |
| Movement commands (n/s/e/w/u/d) | Genre standard since 1978 | LOW | Aliases and short forms expected (n = north) |
| HP/mana/stamina display | Players must always know their vitals at a glance | LOW | Persistent status bar in TUI; always visible |
| Basic combat (attack, flee, kill) | D&D-flavored MUDs are expected to have responsive melee combat | MEDIUM | Attack rolls vs AC, damage rolls — no real-time twitch required |
| Inventory and equipment management | Gear is half the game loop; "inv", "eq", "wear", "remove", "drop" are genre-standard | LOW | Body-slot system: head, neck, body, arms, hands, legs, feet, ring x2, weapon, offhand |
| Local chat (say/emote) | Players in the same room must be able to communicate | LOW | `say`, `emote`/`pose`, `whisper` minimum |
| Global/server-wide chat | Players expect a gossip or global channel to reach the whole server | LOW | Toggleable; newbies expect it; veterans may disable it |
| Help system | New players will type `help` immediately; no help system = instant quit | LOW | Help topics per command, searchable |
| Newbie/tutorial area | First-time players need a safe zone to learn commands before the open world | MEDIUM | Guided room sequence with prompts; low-danger |
| NPC monsters with loot | Combat needs targets that drop gear/gold; empty dungeons are unacceptable | MEDIUM | Spawn tables, respawn timers, basic AI (aggro, patrol) |
| Death handling (soft death) | Players will die; non-punitive recovery is expected for social/exploration focus | LOW | Respawn at bind point with XP debt or gear intact; permadeath is anti-feature for this target |
| Quit/save commands | Players need clean exit; data must save on quit | LOW | Auto-save on quit; periodic world save |
| Account/login system | Multiple characters per account is standard; secure login required | MEDIUM | Username + hashed password at minimum; email optional |

### Differentiators (Competitive Advantage)

Features that set MUT Remastered apart. These align with the core value: social + exploration first.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Rich Unicode TUI with split panels | Most MUDs are raw telnet streams; a structured TUI with persistent status bar, scrollback, and input line is visually modern | HIGH | Ratatui layout: map/compass pane, room description pane, chat pane, character vitals bar |
| Browser-based TUI client (xterm.js) | Eliminates the "download a MUD client" barrier — play in browser instantly | HIGH | WebSocket to game server; xterm.js renders TUI identically to native client |
| Web character armory with 3D models | No MUD has public character profiles with 3D gear visualization; WoW Armory proved players love this | HIGH | blend-ai for model generation; show equipped gear, stats, level, class, achievement history |
| Procedural dungeons with hand-crafted set pieces | Pure procedural = feels hollow; pure hand-crafted = finite content; hybrid is the best of both | HIGH | PCG room/corridor layout with guaranteed boss rooms and hand-written lore at key locations |
| D&D-flavored dice mechanics (visible rolls) | Players love seeing "rolled 17 + 3 STR vs AC 14 — HIT!" — makes math tactile and legible | LOW | Display roll formula in combat log; not hidden behind abstraction |
| Player-to-player inspection | Social glue: "look at [player]" shows their gear and description — encourages showing off character builds | LOW | Requires character display system; high social payoff for low implementation cost |
| Compass/minimap overlay in TUI | Most MUDs have no spatial awareness; a Unicode ASCII minimap of explored rooms gives players a sense of place | MEDIUM | 2D grid, fog of war on unexplored rooms, current position highlighted |
| Area/world lore embedded in rooms | Exploration is motivating when discovery is rewarded; lore in room descriptions creates narrative momentum | MEDIUM | Requires thoughtful world-building; even PCG rooms get templated flavor text |
| Channel system with IC/OOC separation | Roleplay integrity matters; local `say` is IC, global `gossip` is OOC, `newbie` is help channel | LOW | 3-5 named channels; per-character toggle on/off |
| Character biography / backstory field | Social feature: players can write flavor text visible on inspection — rewards character investment | LOW | 500-char bio stored on character; displayed via `look at [player]` |

### Anti-Features (Commonly Requested, Often Problematic)

Features that seem good but create problems or conflict with the project's core value.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| PvP combat / ganking | Players assume MUDs are PvP games historically | Destroys social/exploration atmosphere; toxic dynamics; drives away casual players | Consensual dueling arena as v2 feature only; opt-in flag |
| Permadeath | "Hardcore" appeal; creates tension | Alienates casual and social players; contradicts exploration-first design | Soft death with XP debt and bind-point respawn |
| Strict D&D SRD rules (exact 5e) | D&D familiarity attracts players | License constraints; simulation depth conflicts with MUD pacing; spell slot tracking is tedious in text | D&D-flavored mechanics: ability scores, dice rolls, AC, HP — but simplified spells and combat |
| Real-time chat outside the game (Discord integration) | Players want Discord bridges | Fragments the in-game community; players leave for Discord and never return in-world | Strong in-game channels (OOC gossip, newbie); Discord can link to web armory only |
| Player housing (v1) | Expected from MMO players | Enormous scope; building/interior design systems dwarf core gameplay in complexity | Defer to post-v1; bind point at inn room is sufficient placeholder |
| Trading/auction house (v1) | Economy depth appeals to min-maxers | Requires anti-duping, economy balancing, market UI — multiply scope without validating the core loop | Direct player-to-player trade with `give` command only for v1 |
| Crafting system (v1) | Depth appeal; Alter Aeon has it | Complex balance problem; undermines loot satisfaction if crafted gear is always better | Drop-based loot from dungeons is sufficient for v1 |
| Real-time combat (action-per-second) | MMO players expect responsive combat | Terrible in text; requires complex async architecture; punishes slow typists | Round-based combat with ~2s rounds; typing commands queues into next round |
| Dynamic NPC dialogue (LLM-powered) | AI hype; feels modern | Unpredictable output; breaks lore consistency; massive operational cost; distraction from core social features | Hand-written NPC dialogue trees; LLM integration is a v3 research spike |
| Soundpacks / audio | Alter Aeon uses them; accessibility benefit | Out of scope for terminal-first design; xterm.js can't play audio reliably cross-platform | Focus on visual richness of TUI; audio is a v2 consideration |
| Mobile native app | Accessibility | Web TUI client covers mobile; native app duplicates effort with worse keyboard UX | Ensure xterm.js web client is mobile-responsive (limited but functional) |
| GPU-accelerated terminal rendering | Prettier fonts, smoother animation | Incompatible with xterm and iTerm2 constraints; breaks core terminal compatibility requirement | Ratatui with Unicode/truecolor — visually rich without GPU |

---

## Feature Dependencies

```
Account/Login System
    └──required by──> Character Creation
                          └──required by──> Character Stats/Vitals Display
                          └──required by──> Inventory & Equipment System
                          └──required by──> Player Inspection
                          └──required by──> Web Armory Profile

Persistent World State
    └──required by──> NPC Monsters with Loot
    └──required by──> Procedural Dungeons
    └──required by──> Death Handling

Room Descriptions + Exits
    └──required by──> Movement Commands
    └──required by──> Compass/Minimap TUI Overlay
    └──required by──> Procedural Dungeons

Local Chat (say/emote)
    └──required by──> Channel System (IC/OOC separation)

Basic Combat
    └──required by──> D&D Dice Mechanics (visible rolls)
    └──required by──> NPC Monsters with Loot

TUI Client (Ratatui)
    └──required by──> Compass/Minimap Overlay
    └──required by──> HP/Vitals Status Bar
    └──required by──> Split Chat Pane

Character Creation
    └──enhances──> Web Armory (public profile)
    └──enhances──> Player Inspection

Procedural Dungeons ──enhances──> Hand-Crafted Set Pieces (PCG provides layout; set pieces provide narrative anchors)

Web Armory ──requires──> Account/Login System (auth for public profile claims)
```

### Dependency Notes

- **Account/Login is the root dependency:** Almost every feature touches identity. Build this first.
- **Room system is the world primitive:** Movement, descriptions, chat scoping, dungeon generation, minimap — all depend on rooms existing. Build rooms before combat.
- **TUI client gates visual differentiators:** Minimap, split panes, and vitals bar only exist if the Ratatui layout is built first. Web armory is independent.
- **Web armory is deliberately decoupled:** It reads game state but doesn't affect it. Can be built in parallel with game server after character data schema is stable.
- **Procedural dungeons require room system but NOT hand-crafted content:** PCG and hand-crafted set pieces are additive; PCG ships first, hand-crafted content can be added incrementally.

---

## MVP Definition

### Launch With (v1)

Minimum viable product — what's needed to validate the concept (social + exploration in a beautiful TUI dungeon).

- [ ] Account creation and login — without identity there is no persistence
- [ ] Character creation: name, race (3-4), class (4), ability score generation (3d6 or point buy)
- [ ] Persistent world with hand-crafted starting zone (50-100 rooms) — validate exploration loop
- [ ] Room system: descriptions, exits, look command
- [ ] Movement (n/s/e/w + aliases), HP/mana/stamina display
- [ ] Local chat (say, emote, whisper) — validate social loop
- [ ] Global gossip channel (OOC) — let players find each other
- [ ] Newbie channel + help system — reduce first-session quit rate
- [ ] Basic combat: attack rolls vs AC, HP tracking, flee, death + respawn at bind point
- [ ] NPC monsters with simple loot tables — validate gear loop
- [ ] Inventory + equipment (body slots) — gear must be wearable and inspectable
- [ ] Ratatui TUI with split layout: room description, vitals bar, chat pane, input line
- [ ] Native terminal client (iTerm2 + xterm compatible)
- [ ] xterm.js browser client — removes client install barrier immediately
- [ ] Player-to-player inspection (look at [player]) — social glue

### Add After Validation (v1.x)

Features to add once core game loop is proven and players are retained.

- [ ] Procedural dungeon generation — adds replayability once hand-crafted zone proves the loop
- [ ] Hand-crafted set pieces / boss rooms embedded in PCG dungeons
- [ ] Compass/minimap TUI overlay — add once room graph data structure is stable
- [ ] Web character armory (public profiles, gear display) — add when character data schema is stable
- [ ] 3D gear visualization on armory (blend-ai models) — add after armory HTML/data layer is working
- [ ] Character biography field — low cost social feature, add in v1.x
- [ ] D&D dice roll display in combat log (show "rolled 14 + 2 vs AC 12 — HIT!") — polish pass

### Future Consideration (v2+)

Features to defer until product-market fit is established.

- [ ] Player housing — scope is enormous; needs validated player retention first
- [ ] Trading / auction house — needs population to have an economy worth trading
- [ ] Crafting system — adds meaningful content depth but requires balance work
- [ ] Consensual PvP dueling — needs community norms established first
- [ ] Guild/clan system with ranks and shared bank — social infrastructure for mature community
- [ ] Server-to-server gossip network (Gossip protocol) — inter-MUD chat when there are multiple servers

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Account/Login | HIGH | LOW | P1 |
| Character creation | HIGH | MEDIUM | P1 |
| Room system + movement | HIGH | LOW | P1 |
| Local chat (say/emote) | HIGH | LOW | P1 |
| Basic combat + loot | HIGH | MEDIUM | P1 |
| Inventory + equipment | HIGH | LOW | P1 |
| Ratatui TUI layout | HIGH | MEDIUM | P1 |
| xterm.js browser client | HIGH | MEDIUM | P1 |
| Persistent world state | HIGH | MEDIUM | P1 |
| Help system + newbie area | HIGH | LOW | P1 |
| Global chat channel | MEDIUM | LOW | P1 |
| Player inspection | HIGH | LOW | P1 |
| Procedural dungeon gen | HIGH | HIGH | P2 |
| Minimap/compass overlay | MEDIUM | MEDIUM | P2 |
| Web armory | HIGH | HIGH | P2 |
| 3D gear visualization (blend-ai) | MEDIUM | HIGH | P2 |
| Hand-crafted set pieces | MEDIUM | MEDIUM | P2 |
| D&D dice roll display | MEDIUM | LOW | P2 |
| Character biography | LOW | LOW | P2 |
| Player housing | LOW | HIGH | P3 |
| Auction house / economy | LOW | HIGH | P3 |
| Crafting system | MEDIUM | HIGH | P3 |
| Consensual PvP | LOW | MEDIUM | P3 |
| Guild / clan system | MEDIUM | HIGH | P3 |

**Priority key:**
- P1: Must have for launch
- P2: Should have, add when possible
- P3: Nice to have, future consideration

---

## Competitor Feature Analysis

| Feature | Alter Aeon (1996, active) | Untold Dawn (2024, Rust/Bevy) | MUT Remastered Approach |
|---------|--------------------------|-------------------------------|-------------------------|
| Client | Custom web client + telnet | Browser-first, custom UI | Native Ratatui + xterm.js browser TUI |
| Character system | 6 multiclass, custom levels | Post-cyberpunk RP, freeform | D&D-flavored 4-6 classes, streamlined |
| World | 400+ hand-crafted areas | Hand-crafted, player-built | Hybrid: hand-crafted + PCG dungeons |
| Chat | Gossip, newbie, clan channels | OOC + IC channels | Local/global + IC/OOC separation |
| Combat | Hack-and-slash, real-time | RP-consensual, less combat focus | Round-based, dice-visible, D&D-flavored |
| Social | Clans, emotes, socials | Player factions, emotes | Inspection, emotes, biography, armory |
| Web presence | Informational site | Blog/wiki | Character armory with 3D visualization |
| Accessibility | Screen reader support, Mush-Z | OOC transparency, modern UX | xterm.js lowers barrier; no screen reader scope yet |
| Death | Soft death, respawn | Permadeath | Soft death (bind point respawn) |

---

## Sources

- [MUD Games in 2025 — Inviocean](https://inviocean.com/play/accessibility-as-a-matter-of-survival-mud-games-in-2025/) — Modern MUD accessibility and design expectations
- [Alter Aeon: 25+ Years of Adventure — Writing Games](https://writing-games.org/alter-aeon-mud/) — Longevity features, community, multiclass system
- [Untold Dawn Pre-Alpha](https://blog.untold-dawn.com/) — Modern Rust-based MUD design philosophy
- [MUD Cookbook: Design Meets Implementation — Andrew Zigler](https://www.andrewzigler.com/blog/mud-cookbook-design-meets-implementation/) — Design pitfalls, persistence decisions, progression balance
- [The 5 Basic MUD Styles and You — Writing Games](https://writing-games.org/mud-styles-and-player-types/) — MUD genre taxonomy, social vs hack-and-slash
- [WoW Armory 3D Model Viewer — Blizzplanet](https://warcraft.blizzplanet.com/blog/comments/wow_armory_3d_model_viewer_and_character_feeds) — Armory feature reference for web character profiles
- [GMCP Protocol — MudVault](https://mudvault.org/protocols) — Modern MUD protocol landscape
- [Mudlet Supported Protocols](https://wiki.mudlet.org/w/Manual:Supported_Protocols) — Client-side protocol expectations (GMCP, MSDP, MSSP)
- [Permanent Death — Muds Wiki](https://muds.fandom.com/wiki/Permanent_death) — Death penalty design considerations
- [Multi-User Dungeon — Wikipedia](https://en.wikipedia.org/wiki/Multi-user_dungeon) — Genre history, feature taxonomy
- [MudVerse Rankings](https://www.mudverse.com/rankings) — Active MUD ecosystem reference

---

*Feature research for: Multi-User Text Dungeon (MUD) — social/exploration-first, D&D-flavored, Rust + Ratatui + xterm.js*
*Researched: 2026-03-23*
