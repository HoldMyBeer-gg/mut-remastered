# Phase 03: Character and Combat - Validation Strategy

**Phase:** 03-character-and-combat
**Created:** 2026-03-24

## Success Criteria Mapping

From ROADMAP.md Phase 3 success criteria:

| # | Criterion | Requirements | Validated By |
|---|-----------|-------------|--------------|
| 1 | Player can create a character by choosing race, class, allocating ability scores, and selecting gender — character appears in world immediately | AUTH-03, AUTH-04, AUTH-05, AUTH-06, AUTH-07 | Integration test: register → create character → select → get RoomDescription |
| 2 | Combat log shows dice roll results in plain language and resolves in ~2-second rounds | CMBT-01, CMBT-02 | Integration test: attack NPC → verify CombatLog format + timing |
| 3 | HP, mana, and stamina are always visible without typing a command | CHAR-01 | Integration test: any state-changing action → Vitals message received |
| 4 | Player can pick up, drop, and equip gear to named body slots — equipped gear changes stats | CHAR-02, CHAR-03, CHAR-04 | Integration test: get item → equip → stats change → unequip → stats revert |
| 5 | Death respawns at bind point with gear intact and XP debt | CMBT-06 | Integration test: die in combat → verify position change, gear still present, XP reduced |

## Requirement Coverage

| Requirement | Plan | Test |
|-------------|------|------|
| AUTH-03 (multiple characters per account) | 03-01 | test_multiple_characters_per_account |
| AUTH-04 (race selection) | 03-01 | test_character_create_valid_race |
| AUTH-05 (class selection) | 03-01 | test_character_create_valid_class |
| AUTH-06 (ability score allocation) | 03-01 | test_point_buy_validation |
| AUTH-07 (gender selection) | 03-01 | test_character_create_gender |
| CHAR-01 (vitals always visible) | 03-02 | test_vitals_sent_on_combat_action |
| CHAR-02 (inventory management) | 03-03 | test_get_drop_items |
| CHAR-03 (equip/unequip to slots) | 03-03 | test_equip_unequip_slots |
| CHAR-04 (equipment affects stats) | 03-03 | test_equipment_changes_stats |
| CHAR-05 (500-char biography) | 03-03 | test_bio_set_and_read |
| CMBT-01 (D&D dice mechanics visible) | 03-02 | test_combat_log_format |
| CMBT-02 (round-based ~2s) | 03-02 | test_combat_round_timing |
| CMBT-03 (NPC spawn with respawn) | 03-02 | test_npc_spawn_and_respawn |
| CMBT-04 (NPC aggro) | 03-02 | test_npc_aggro_on_enter |
| CMBT-05 (loot drops) | 03-03 | test_monster_drops_loot |
| CMBT-06 (soft death) | 03-02 | test_death_respawn_xp_debt |

## Nyquist Validation

**Each success criterion must be tested by at least 2 independent signals:**

| Criterion | Signal 1 | Signal 2 |
|-----------|----------|----------|
| Character creation | DB row exists with correct race/class/scores | RoomDescription received after select |
| Combat dice display | CombatLog string matches regex `rolled \d+ \+ \d+.*vs AC \d+` | Round timing within 1.5-2.5s window |
| Vitals visibility | Vitals message received after combat action | Vitals message received after equip action |
| Inventory/equipment | StatsResult shows AC change after equip | InventoryList reflects item moved |
| Soft death | Position changed to bind point | XP reduced by ~10%; gear count unchanged |
