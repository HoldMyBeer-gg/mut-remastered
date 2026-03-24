# Phase 03: Character and Combat - Research

**Researched:** 2026-03-24
**Domain:** D&D-lite rules subset, character creation, combat engine, NPC AI, inventory systems
**Confidence:** HIGH

---

## Summary

Phase 3 adds the RPG core to MUT: character creation (8 races, 4-6 classes, 6 ability scores), D&D-lite combat (d20+mod vs AC, damage dice, ~2s rounds), NPC monsters with aggro/patrol AI, inventory and equipment with body slots, and soft death with XP debt. The combat system runs as a fixed-tick Tokio task that processes queued player actions per round, while the rest of the world remains event-driven.

**Primary architecture:** A `combat` module manages active encounters as state machines. Each combat instance tracks combatants, initiative order, round number, and queued actions. The combat tick task iterates all active combats every ~2 seconds, resolving queued actions, applying damage, checking deaths, and broadcasting results. Non-combat world commands continue to work via the existing event-driven pattern.

**Key new crate dependencies:** `rand` 0.9 (dice rolling). All other needs are met by existing workspace dependencies.

---

## MUT D&D-Lite Rules Subset

This is the definitive rules reference for Phase 3. All combat code implements these formulas.

### Ability Scores

Six abilities: **STR**, **DEX**, **CON**, **INT**, **WIS**, **CHA**

- Range: 3-20 (base), can exceed 20 with equipment
- **Modifier** = `(score - 10) / 2` (integer division, rounds toward negative infinity)
  - Score 8 → modifier -1
  - Score 10 → modifier 0
  - Score 12 → modifier +1
  - Score 14 → modifier +2
  - Score 16 → modifier +3
  - Score 18 → modifier +4
  - Score 20 → modifier +5

### Character Creation

**Point Buy System (27 points):**
- All scores start at 8
- Costs: 8→9 = 1pt, 9→10 = 1pt, 10→11 = 1pt, 11→12 = 1pt, 12→13 = 1pt, 13→14 = 2pt, 14→15 = 2pt
- Max base score from point buy: 15 (before racial bonus)
- Total: 27 points to distribute

**Races (8):**

| Race | Bonus | HP Bonus | Flavor |
|------|-------|----------|--------|
| Human | +1 to two abilities of choice | — | Versatile, adaptable |
| Elf | +2 DEX | — | Graceful, perceptive |
| Dwarf | +2 CON | +1 HP/level | Stout, resilient |
| Halfling | +2 DEX | — | Lucky, nimble |
| Orc | +2 STR, +1 CON | — | Powerful, enduring |
| Gnome | +2 INT | — | Clever, inventive |
| Half-Elf | +2 CHA, +1 to one ability of choice | — | Diplomatic, versatile |
| Tiefling | +2 CHA, +1 INT | — | Cunning, fiendish heritage |

**Classes (5):**

| Class | Hit Die | Primary Ability | Armor | Weapon | Description |
|-------|---------|-----------------|-------|--------|-------------|
| Warrior | d10 | STR | Heavy | All melee + shields | Frontline tank/damage |
| Ranger | d8 | DEX | Medium | Ranged + finesse melee | Agile scout/archer |
| Cleric | d8 | WIS | Medium + shields | Maces/staves | Healer (self-heal ability) |
| Mage | d6 | INT | None | Staves/daggers | Highest burst damage (blast ability) |
| Rogue | d8 | DEX | Light | Finesse + daggers | Sneak attack bonus on first hit |

**Class Abilities (one per class, simple):**

| Class | Ability | Effect | Cooldown |
|-------|---------|--------|----------|
| Warrior | Power Strike | Next attack deals double damage | 5 rounds |
| Ranger | Aimed Shot | +5 to hit on next attack | 3 rounds |
| Cleric | Heal | Restore 2d8+WIS_mod HP to self | 5 rounds |
| Mage | Arcane Blast | Deal 3d6+INT_mod damage, ignores AC (auto-hit) | 5 rounds |
| Rogue | Sneak Attack | First attack each combat deals extra 2d6 damage | Automatic (once per combat) |

**Derived Stats at Level 1:**
- **HP** = Hit Die max + CON modifier (+ racial bonus if any)
- **Mana** = 10 + INT modifier × 2 (used by Mage abilities; other classes still have mana pool for future use)
- **Stamina** = 10 + CON modifier × 2 (used by Warrior/Ranger/Rogue abilities)
- **AC** = 10 + DEX modifier + armor bonus
- **Initiative** = d20 + DEX modifier

### Combat Mechanics

**Starting Combat:**
- Player types `attack <target>` to initiate
- NPC aggro: some NPCs auto-attack when a player enters their room
- Initiative: all combatants roll d20 + DEX modifier; act in descending order

**Attack Roll:**
- Roll d20 + ability modifier + proficiency bonus (fixed at +2 for level 1)
- Melee: d20 + STR mod + 2 (or DEX mod for finesse weapons)
- Ranged: d20 + DEX mod + 2
- Natural 20 = critical hit (double damage dice)
- Natural 1 = automatic miss

**Damage:**
- Weapon damage die + ability modifier
- Example weapons: Shortsword 1d6+STR, Longbow 1d8+DEX, Staff 1d6+STR, Dagger 1d4+DEX

**Round Resolution (CMBT-02: ~2 second rounds):**
1. Combat tick fires every 2 seconds
2. For each active combat: process queued actions in initiative order
3. If a combatant has no queued action, they auto-attack their last target
4. Apply damage, check for deaths
5. Broadcast combat log to room

**Death (CMBT-06):**
- When HP reaches 0, the character dies
- Respawn at bind point (default: starting_village:market_square) after 5 seconds
- Gear intact, no item loss
- XP debt: lose 10% of current XP (can go negative at level 1, preventing level-down)
- Death message broadcast to room

### NPC Monsters

**Spawn System (CMBT-03):**
- Monsters defined in zone TOML files as spawn tables per room
- Each spawn entry: monster_id, count, respawn_timer_secs
- On server start: spawn initial monsters
- On monster death: start respawn timer, spawn new instance when timer expires

**Monster AI (CMBT-04):**
- **Passive:** Stands in room, doesn't attack unless attacked
- **Aggressive:** Attacks any player who enters the room (after a short delay)
- **Patrol:** Moves between rooms on a timer (future — Phase 3 implements aggro only, patrol as stretch)

**Loot Tables (CMBT-05):**
- Each monster has a loot table: array of (item_id, drop_chance_percent)
- On death: roll each entry; drop items into room for players to pick up
- Gold: min-max range, always drops

### Inventory & Equipment

**Body Slots (CHAR-03):**
- head, neck, body, arms, hands, legs, feet, ring_1, ring_2, weapon, offhand
- Each slot holds one item
- Equipping an item to an occupied slot swaps the old item to inventory

**Item Types:**
- Weapon: damage die, damage type, ability (STR or DEX)
- Armor: AC bonus, slot, weight class (light/medium/heavy)
- Ring/Accessory: stat bonuses
- Consumable: (future — not in Phase 3)

**Inventory Commands (CHAR-02):**
- `inventory` / `inv` — list carried items
- `get <item>` / `pick up <item>` — pick up from room floor
- `drop <item>` — drop to room floor
- `equip <item>` — equip to appropriate slot
- `unequip <slot>` — move equipped item to inventory

**Stat Effects (CHAR-04):**
- Equipment bonuses are additive
- Recalculated on equip/unequip
- AC = 10 + DEX mod + sum of armor bonuses
- Attack bonus = ability mod + proficiency + weapon bonus (if any)

---

## New Protocol Messages

### Character Namespace (extend auth or new NS_CHAR)

**ClientMsg additions:**
```
CharacterList                      — list all characters on account
CharacterCreate { name, race, class, gender, ability_scores }
CharacterSelect { character_id }   — enter world as this character
```

**ServerMsg additions:**
```
CharacterListResult { characters: Vec<CharacterSummary> }
CharacterCreateOk { character_id }
CharacterCreateFail { reason }
CharacterSelected { character_id, name }
```

### World/Combat Messages (extend world namespace)

**ClientMsg additions:**
```
Attack { target: String }
Flee
UseAbility { ability_name: String }
Inventory
GetItem { target: String }
DropItem { target: String }
Equip { item_name: String }
Unequip { slot: String }
Stats                              — view own character stats
Bio { text: String }               — set biography (CHAR-05)
```

**ServerMsg additions:**
```
Vitals { hp, max_hp, mana, max_mana, stamina, max_stamina, xp, level }
CombatLog { entries: Vec<String> }
CombatStart { combatants: Vec<String> }
CombatEnd { result: String }
InventoryList { items: Vec<ItemSummary>, equipped: HashMap<String, ItemSummary> }
GetItemOk { item_name }
DropItemOk { item_name }
EquipOk { item_name, slot }
UnequipOk { slot, item_name }
StatsResult { ... }
BioOk
ActionFail { reason: String }      — generic failure for combat/inventory actions
```

---

## Database Schema (New Migrations)

### 005_characters.sql
```sql
CREATE TABLE characters (
    id          TEXT PRIMARY KEY NOT NULL,
    account_id  TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    name        TEXT UNIQUE NOT NULL COLLATE NOCASE,
    race        TEXT NOT NULL,
    class       TEXT NOT NULL,
    gender      TEXT NOT NULL,
    level       INTEGER NOT NULL DEFAULT 1,
    xp          INTEGER NOT NULL DEFAULT 0,
    hp          INTEGER NOT NULL,
    max_hp      INTEGER NOT NULL,
    mana        INTEGER NOT NULL,
    max_mana    INTEGER NOT NULL,
    stamina     INTEGER NOT NULL,
    max_stamina INTEGER NOT NULL,
    str_score   INTEGER NOT NULL,
    dex_score   INTEGER NOT NULL,
    con_score   INTEGER NOT NULL,
    int_score   INTEGER NOT NULL,
    wis_score   INTEGER NOT NULL,
    cha_score   INTEGER NOT NULL,
    bio         TEXT NOT NULL DEFAULT '',
    bind_point  TEXT NOT NULL DEFAULT 'starting_village:market_square',
    created_at  INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE INDEX idx_characters_account_id ON characters(account_id);
```

### 006_inventory.sql
```sql
CREATE TABLE items (
    id          TEXT PRIMARY KEY NOT NULL,
    character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    template_id TEXT NOT NULL,
    slot        TEXT,  -- NULL = in inventory, non-NULL = equipped to that slot
    created_at  INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE INDEX idx_items_character_id ON items(character_id);

CREATE TABLE room_items (
    id          TEXT PRIMARY KEY NOT NULL,
    room_id     TEXT NOT NULL,
    template_id TEXT NOT NULL,
    created_at  INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE INDEX idx_room_items_room_id ON room_items(room_id);
```

### 007_monsters.sql
```sql
CREATE TABLE active_monsters (
    id          TEXT PRIMARY KEY NOT NULL,
    template_id TEXT NOT NULL,
    room_id     TEXT NOT NULL,
    hp          INTEGER NOT NULL,
    max_hp      INTEGER NOT NULL,
    spawned_at  INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE INDEX idx_active_monsters_room_id ON active_monsters(room_id);
```

### Migration: player_positions account_id → character_id
- The existing `player_positions` table uses `account_id` as PK
- After character creation, position tracking should use `character_id`
- Migration: rename column or create new table with character_id FK

---

## Data Files

### Monster Templates (TOML)
```toml
# world/data/monsters.toml
[[monsters]]
id = "rat"
name = "Giant Rat"
hp = 8
ac = 12
attack_bonus = 3
damage = "1d4+1"
xp_value = 25
aggro = "passive"
loot = [
    { item = "rat_tail", chance = 50 },
    { gold_min = 1, gold_max = 5 },
]

[[monsters]]
id = "goblin"
name = "Goblin Scout"
hp = 15
ac = 13
attack_bonus = 4
damage = "1d6+2"
xp_value = 50
aggro = "aggressive"
loot = [
    { item = "rusty_shortsword", chance = 30 },
    { item = "leather_scraps", chance = 50 },
    { gold_min = 5, gold_max = 15 },
]
```

### Item Templates (TOML)
```toml
# world/data/items.toml
[[items]]
id = "rusty_shortsword"
name = "Rusty Shortsword"
kind = "weapon"
slot = "weapon"
damage = "1d6"
ability = "str"
description = "A pitted blade that's seen better days."

[[items]]
id = "leather_armor"
name = "Leather Armor"
kind = "armor"
slot = "body"
ac_bonus = 2
weight_class = "light"
description = "Supple leather reinforced with studs."

[[items]]
id = "iron_ring"
name = "Iron Ring of Fortitude"
kind = "accessory"
slot = "ring"
stat_bonuses = { con = 1 }
description = "A plain iron band that thrums with faint warmth."
```

### Zone Spawn Tables (added to zone TOML)
```toml
# Added to world/zones/starting_village/zone.toml
[[rooms.spawns]]
monster = "rat"
count = 2
respawn_secs = 120
```

---

## Architecture Patterns

### Pattern 1: Combat State Machine

Combat is managed as a standalone state machine, separate from the world event loop:

```
enum CombatState {
    Rolling,     // Collecting initiative, about to start
    InProgress,  // Active combat rounds
    Ending,      // Cleanup (loot generation, XP award)
}

struct CombatInstance {
    id: Uuid,
    room_id: RoomId,
    combatants: Vec<Combatant>,  // players + NPCs
    initiative_order: Vec<usize>,
    current_round: u32,
    state: CombatState,
    queued_actions: HashMap<String, CombatAction>,
}
```

A `CombatManager` holds all active combats in a `HashMap<Uuid, CombatInstance>`. The combat tick task (2-second interval) calls `manager.tick()` which processes all active combats.

### Pattern 2: Character Loading Flow

After login:
1. Server sends `CharacterListResult` with account's characters
2. Player sends `CharacterSelect { character_id }`
3. Server loads character from DB, places in world, subscribes to room
4. Server sends `Vitals` and `RoomDescription`

This adds a "character selection" state between "logged in" and "in world".

### Pattern 3: Dice Rolling

```rust
use rand::Rng;

fn roll_d20() -> i32 { rand::rng().random_range(1..=20) }
fn roll_dice(count: u32, sides: u32) -> i32 {
    let mut rng = rand::rng();
    (0..count).map(|_| rng.random_range(1..=sides as i32)).sum()
}
```

### Pattern 4: Combat Log Formatting (CMBT-01)

```
"Grok rolled 14 + 2 STR vs AC 12 — HIT! 7 damage (1d6+3)"
"Goblin Scout rolled 8 + 4 vs AC 16 — MISS!"
"CRITICAL HIT! Grok rolled 20 + 2 STR vs AC 12 — 14 damage (2d6+3)"
"Grok has been slain! Respawning at Briarhollow Village..."
```

---

## New Crate Dependencies

| Crate | Version | Purpose | Why |
|-------|---------|---------|-----|
| rand | 0.9 | Dice rolling, loot table RNG | Standard Rust RNG; already in STACK.md recommendation |

All other needs (serde, sqlx, tokio, postcard, toml, tracing, uuid) are already in the workspace.

**server/Cargo.toml addition:**
```toml
rand = "0.9"
```

---

## Plan Breakdown

### Plan 03-01: Character Creation + DB Schema
- New migrations: characters table, update player_positions to character_id
- Protocol messages: CharacterCreate, CharacterList, CharacterSelect
- Race/class/ability score definitions as Rust enums/structs
- Point-buy validation
- Character creation handler
- Character selection flow in ConnectionActor
- DB persistence for characters

### Plan 03-02: Combat Engine + NPC Spawns
- Monster template TOML loader
- Combat state machine (CombatInstance, CombatManager)
- Fixed-tick combat loop (2-second Tokio interval)
- Attack/flee/use-ability command handlers
- D&D-lite dice mechanics (d20+mod vs AC, damage)
- Combat log message formatting
- NPC aggro system
- Death/respawn with XP debt
- Vitals protocol messages

### Plan 03-03: Inventory/Equipment + Integration Tests
- Item template TOML loader
- Inventory DB schema (items table, room_items table)
- Inventory commands: get, drop, equip, unequip, inventory, stats, bio
- Equipment stat calculation (AC from armor, damage from weapon)
- Loot drop system (monster death → items on floor)
- Gold system (simple integer on character)
- Integration tests for all 16 Phase 3 requirements
- Test helpers extension for character creation and combat flows

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Combat tick contention with world RwLock | Medium | High | Combat manager has its own lock; only reads world for room data; writes are short |
| player_positions migration breaks existing tests | Medium | Medium | Careful migration; keep backward compat or update all tests in Plan 1 |
| Combat balance (monsters too hard/easy) | High | Low | Tunable via TOML data files; can adjust without code changes |
| Protocol namespace collision | Low | Medium | Add NS_CHAR namespace byte if needed; or extend NS_AUTH/NS_WORLD carefully |

---

*Phase: 03-character-and-combat*
*Research completed: 2026-03-24*
