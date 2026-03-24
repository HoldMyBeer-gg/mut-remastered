//! CombatManager: holds all active combat instances and processes rounds.

use std::collections::HashMap;

use crate::character::types::ability_modifier;
use crate::combat::engine::{format_combat_log, resolve_attack, roll_damage};
use crate::combat::types::*;
use crate::world::types::RoomId;

use rand::Rng;

/// Manages all active combats across all rooms.
#[derive(Debug, Default)]
pub struct CombatManager {
    /// Active combat per room (at most one combat per room).
    pub combats: HashMap<RoomId, CombatInstance>,
}

impl CombatManager {
    pub fn new() -> Self {
        Self {
            combats: HashMap::new(),
        }
    }

    /// Start a new combat in a room. Combatants are sorted by initiative (descending).
    pub fn start_combat(
        &mut self,
        room_id: RoomId,
        mut combatants: Vec<CombatantInfo>,
    ) -> Vec<String> {
        // Sort by initiative descending (highest goes first)
        combatants.sort_by(|a, b| b.initiative.cmp(&a.initiative));

        let names: Vec<String> = combatants.iter().map(|c| c.name.clone()).collect();

        let instance = CombatInstance::new(room_id.clone(), combatants);
        self.combats.insert(room_id, instance);
        names
    }

    /// Queue an action for a combatant in the given room's combat.
    pub fn queue_action(
        &mut self,
        room_id: &RoomId,
        combatant_id: CombatantId,
        action: CombatAction,
    ) {
        if let Some(combat) = self.combats.get_mut(room_id) {
            // Track last target for auto-attack
            if let CombatAction::Attack { ref target } = action {
                combat
                    .last_targets
                    .insert(combatant_id.clone(), target.clone());
            }
            combat.queued_actions.insert(combatant_id, action);
        }
    }

    /// Check if a room has an active combat.
    pub fn has_combat(&self, room_id: &RoomId) -> bool {
        self.combats.contains_key(room_id)
    }

    /// Check if a combatant is in any active combat.
    pub fn find_combat_for_player(&self, character_id: &str) -> Option<RoomId> {
        let player_id = CombatantId::Player(character_id.to_string());
        self.combats
            .iter()
            .find(|(_, combat)| combat.has_combatant(&player_id))
            .map(|(room_id, _)| room_id.clone())
    }

    /// Process one round of all active combats.
    ///
    /// Returns a map of room_id -> RoundResult for each room that had combat.
    /// Caller is responsible for dispatching events (sending messages, updating DB, etc.).
    pub fn tick(
        &mut self,
        active_monsters: &mut HashMap<RoomId, Vec<ActiveMonster>>,
        player_stats: &HashMap<String, PlayerCombatStats>,
    ) -> HashMap<RoomId, RoundResult> {
        let mut results = HashMap::new();
        let mut rooms_to_remove = Vec::new();

        for (room_id, combat) in &mut self.combats {
            combat.round += 1;

            let mut log_entries = Vec::new();
            let vitals_updates = Vec::new();
            let mut deaths = Vec::new();
            let mut monster_kills = Vec::new();
            let mut fled = Vec::new();

            // Process combatants in initiative order
            let combatant_order: Vec<CombatantInfo> = combat.combatants.clone();

            for combatant in &combatant_order {
                // Skip dead combatants
                let is_alive = match &combatant.id {
                    CombatantId::Player(char_id) => {
                        player_stats.get(char_id).map(|s| s.hp > 0).unwrap_or(false)
                    }
                    CombatantId::Monster(mon_id) => {
                        if let Some(monsters) = active_monsters.get(room_id) {
                            monsters.iter().any(|m| m.id == *mon_id && m.is_alive())
                        } else {
                            false
                        }
                    }
                };
                if !is_alive {
                    continue;
                }

                // Get action: queued action, or auto-attack last target
                let action = combat
                    .queued_actions
                    .remove(&combatant.id)
                    .or_else(|| {
                        combat.last_targets.get(&combatant.id).map(|target| {
                            CombatAction::Attack {
                                target: target.clone(),
                            }
                        })
                    });

                let action = match action {
                    Some(a) => a,
                    None => {
                        // Monster with no target: pick first player
                        if let CombatantId::Monster(_) = &combatant.id {
                            let first_player = combat.combatants.iter().find_map(|c| {
                                if let CombatantId::Player(id) = &c.id {
                                    Some(CombatantId::Player(id.clone()))
                                } else {
                                    None
                                }
                            });
                            if let Some(target) = first_player {
                                combat
                                    .last_targets
                                    .insert(combatant.id.clone(), target.clone());
                                CombatAction::Attack { target }
                            } else {
                                continue;
                            }
                        } else {
                            continue; // Player with no action: skip
                        }
                    }
                };

                match action {
                    CombatAction::Attack { ref target } => {
                        // Resolve attack based on attacker type
                        match &combatant.id {
                            CombatantId::Player(char_id) => {
                                if let Some(stats) = player_stats.get(char_id) {
                                    let attack_bonus = stats.attack_bonus;
                                    let defender_ac = get_target_ac(target, active_monsters, room_id, player_stats);
                                    let result = resolve_attack(attack_bonus, defender_ac);

                                    let damage = if result.is_hit() {
                                        let (dmg, desc) = roll_damage(
                                            stats.damage_dice,
                                            stats.damage_sides,
                                            stats.damage_bonus,
                                            result.is_crit(),
                                        );
                                        Some((dmg, desc))
                                    } else {
                                        None
                                    };

                                    let log = format_combat_log(
                                        &combatant.name,
                                        &result,
                                        attack_bonus,
                                        defender_ac,
                                        stats.ability_label,
                                        damage.clone(),
                                    );
                                    log_entries.push(log);

                                    // Apply damage to target
                                    if let Some((dmg, _)) = damage {
                                        apply_damage_to_target(
                                            target,
                                            dmg,
                                            active_monsters,
                                            room_id,
                                            player_stats,
                                            &mut log_entries,
                                            &mut deaths,
                                            &mut monster_kills,
                                            &combat.player_participants,
                                        );
                                    }
                                }
                            }
                            CombatantId::Monster(mon_id) => {
                                if let Some(monsters) = active_monsters.get(room_id) {
                                    if let Some(monster) = monsters.iter().find(|m| m.id == *mon_id) {
                                        let attack_bonus = monster.attack_bonus;
                                        let defender_ac = get_target_ac(target, active_monsters, room_id, player_stats);
                                        let result = resolve_attack(attack_bonus, defender_ac);

                                        let damage = if result.is_hit() {
                                            let (dmg, desc) = roll_damage(
                                                monster.damage_dice,
                                                monster.damage_sides,
                                                monster.damage_bonus,
                                                result.is_crit(),
                                            );
                                            Some((dmg, desc))
                                        } else {
                                            None
                                        };

                                        let log = format_combat_log(
                                            &monster.name,
                                            &result,
                                            attack_bonus,
                                            defender_ac,
                                            "",
                                            damage.clone(),
                                        );
                                        log_entries.push(log);

                                        // Apply damage
                                        if let Some((dmg, _)) = damage {
                                            apply_damage_to_target(
                                                target,
                                                dmg,
                                                active_monsters,
                                                room_id,
                                                player_stats,
                                                &mut log_entries,
                                                &mut deaths,
                                                &mut monster_kills,
                                                &combat.player_participants,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    CombatAction::Flee => {
                        // 50% chance to flee
                        let flee_roll = rand::rng().random_range(1..=100);
                        if flee_roll <= 50 {
                            if let CombatantId::Player(char_id) = &combatant.id {
                                log_entries.push(format!("{} fled from combat!", combatant.name));
                                fled.push(char_id.clone());
                            }
                        } else {
                            log_entries.push(format!(
                                "{} tried to flee but couldn't escape!",
                                combatant.name
                            ));
                        }
                    }
                    CombatAction::UseAbility { ability_name, target } => {
                        // TODO: Implement class abilities in future iteration
                        log_entries.push(format!(
                            "{} tries to use {} but it's not yet implemented!",
                            combatant.name, ability_name
                        ));
                        let _ = target; // suppress warning
                    }
                }
            }

            // Remove dead combatants and fled players from combat
            for death in &deaths {
                combat.remove_combatant(&CombatantId::Player(death.character_id.clone()));
            }
            for kill in &monster_kills {
                combat.remove_combatant(&CombatantId::Monster(kill.monster_id.clone()));
            }
            for char_id in &fled {
                combat.remove_combatant(&CombatantId::Player(char_id.clone()));
            }

            // Check if combat should end
            let combat_ended = !combat.has_players() || !combat.has_monsters();
            let end_message = if combat_ended {
                if !combat.has_monsters() && combat.has_players() {
                    Some("Victory! All enemies have been defeated.".to_string())
                } else if !combat.has_players() {
                    Some("The battle is over.".to_string())
                } else {
                    Some("Combat has ended.".to_string())
                }
            } else {
                None
            };

            if combat_ended {
                rooms_to_remove.push(room_id.clone());
            }

            results.insert(
                room_id.clone(),
                RoundResult {
                    log_entries,
                    vitals_updates,
                    deaths,
                    monster_kills,
                    fled,
                    combat_ended,
                    end_message,
                },
            );
        }

        // Remove ended combats
        for room_id in rooms_to_remove {
            self.combats.remove(&room_id);
        }

        results
    }
}

/// Cached player stats for combat resolution (avoids holding world lock during combat tick).
#[derive(Debug, Clone)]
pub struct PlayerCombatStats {
    pub character_id: String,
    pub name: String,
    pub hp: i32,
    pub max_hp: i32,
    pub mana: i32,
    pub max_mana: i32,
    pub stamina: i32,
    pub max_stamina: i32,
    pub xp: i32,
    pub level: i32,
    pub ac: i32,
    pub attack_bonus: i32,
    pub damage_dice: u32,
    pub damage_sides: u32,
    pub damage_bonus: i32,
    pub ability_label: &'static str,
}

/// Get the AC of a target combatant.
fn get_target_ac(
    target: &CombatantId,
    active_monsters: &HashMap<RoomId, Vec<ActiveMonster>>,
    room_id: &RoomId,
    player_stats: &HashMap<String, PlayerCombatStats>,
) -> i32 {
    match target {
        CombatantId::Player(char_id) => {
            player_stats.get(char_id).map(|s| s.ac).unwrap_or(10)
        }
        CombatantId::Monster(mon_id) => {
            if let Some(monsters) = active_monsters.get(room_id) {
                monsters.iter().find(|m| m.id == *mon_id).map(|m| m.ac).unwrap_or(10)
            } else {
                10
            }
        }
    }
}

/// Apply damage to a target, handling death for both players and monsters.
fn apply_damage_to_target(
    target: &CombatantId,
    damage: i32,
    active_monsters: &mut HashMap<RoomId, Vec<ActiveMonster>>,
    room_id: &RoomId,
    player_stats: &HashMap<String, PlayerCombatStats>,
    log_entries: &mut Vec<String>,
    deaths: &mut Vec<DeathEvent>,
    monster_kills: &mut Vec<MonsterKill>,
    player_participants: &[String],
) {
    match target {
        CombatantId::Monster(mon_id) => {
            if let Some(monsters) = active_monsters.get_mut(room_id) {
                if let Some(monster) = monsters.iter_mut().find(|m| m.id == *mon_id) {
                    monster.hp -= damage;
                    if monster.hp <= 0 {
                        monster.hp = 0;
                        log_entries.push(format!("{} has been slain!", monster.name));
                        monster_kills.push(MonsterKill {
                            monster_id: mon_id.clone(),
                            template_id: monster.template_id.clone(),
                            room_id: room_id.clone(),
                            xp_value: monster.xp_value,
                            participants: player_participants.to_vec(),
                        });
                    }
                }
            }
        }
        CombatantId::Player(char_id) => {
            // Player damage is tracked via player_stats which is read-only here.
            // The actual HP modification happens in the tick loop caller.
            // We just record the death event if HP would reach 0.
            if let Some(stats) = player_stats.get(char_id) {
                if stats.hp - damage <= 0 {
                    log_entries.push(format!("{} has been slain!", stats.name));
                    deaths.push(DeathEvent {
                        character_id: char_id.clone(),
                        character_name: stats.name.clone(),
                    });
                }
            }
        }
    }
}

/// Build player combat stats from character DB row data.
/// This is called before a combat tick to snapshot player state.
pub fn build_player_combat_stats(
    character_id: &str,
    name: &str,
    hp: i32,
    max_hp: i32,
    mana: i32,
    max_mana: i32,
    stamina: i32,
    max_stamina: i32,
    xp: i32,
    level: i32,
    str_score: u8,
    dex_score: u8,
    _con_score: u8,
    _int_score: u8,
    _wis_score: u8,
    _cha_score: u8,
    class: &str,
) -> PlayerCombatStats {
    // Determine attack ability based on class
    let (attack_mod, ability_label) = match class {
        "ranger" | "rogue" => (ability_modifier(dex_score), "DEX"),
        _ => (ability_modifier(str_score), "STR"),
    };

    // Base proficiency bonus (+2 at level 1)
    let proficiency = 2;

    // Default weapon: fists (1d4 + ability mod)
    // TODO: Use equipped weapon in Plan 03-03
    let (damage_dice, damage_sides, damage_bonus) = (1u32, 4u32, attack_mod);

    PlayerCombatStats {
        character_id: character_id.to_string(),
        name: name.to_string(),
        hp,
        max_hp,
        mana,
        max_mana,
        stamina,
        max_stamina,
        xp,
        level,
        ac: 10 + ability_modifier(dex_score), // Base AC, no armor yet (Plan 03-03)
        attack_bonus: attack_mod + proficiency,
        damage_dice,
        damage_sides,
        damage_bonus,
        ability_label,
    }
}
