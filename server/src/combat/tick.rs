//! Async combat tick loop and monster respawn management.
//!
//! Runs as a background Tokio task, ticking every 2 seconds.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use sqlx::SqlitePool;
use tokio::sync::{RwLock, broadcast};
use tracing::{debug, warn};

use crate::combat::manager::{CombatManager, PlayerCombatStats, build_player_combat_stats};
use crate::combat::types::*;
use crate::world::types::{RoomId, WorldEvent};

/// Run the combat tick loop. Called once at server startup via `tokio::spawn`.
///
/// Every 2 seconds:
/// 1. Snapshot player combat stats from DB
/// 2. Process one round of all active combats
/// 3. Dispatch results: broadcast combat logs, send vitals, handle deaths
/// 4. Check and process monster respawn timers
pub async fn combat_tick_loop(
    combat_manager: Arc<RwLock<CombatManager>>,
    active_monsters: Arc<RwLock<HashMap<RoomId, Vec<ActiveMonster>>>>,
    world: Arc<RwLock<crate::world::types::World>>,
    room_channels: Arc<RwLock<HashMap<RoomId, broadcast::Sender<WorldEvent>>>>,
    monster_templates: Arc<HashMap<String, MonsterTemplate>>,
    respawn_timers: Arc<RwLock<Vec<RespawnTimer>>>,
    db: SqlitePool,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(2));

    loop {
        interval.tick().await;

        // 1. Check if there are any active combats
        let has_combats = {
            let mgr = combat_manager.read().await;
            !mgr.combats.is_empty()
        };

        if has_combats {
            // 2. Snapshot player stats from DB for all players in combat
            let mut player_stats = snapshot_player_stats(&combat_manager, &db).await;

            // 3. Process one round
            let results = {
                let mut mgr = combat_manager.write().await;
                let mut monsters = active_monsters.write().await;
                mgr.tick(&mut monsters, &mut player_stats)
            };

            // 3b. Persist player HP changes to DB
            for (char_id, stats) in &player_stats {
                let _ = sqlx::query(
                    "UPDATE characters SET hp = ? WHERE id = ?"
                )
                .bind(stats.hp)
                .bind(char_id)
                .execute(&db)
                .await;
            }

            // 4. Dispatch results
            for (room_id, result) in &results {
                // Broadcast combat log to room
                if !result.log_entries.is_empty() {
                    let channels = room_channels.read().await;
                    if let Some(sender) = channels.get(room_id) {
                        let log_text = result.log_entries.join("\n");
                        let _ = sender.send(WorldEvent {
                            message: log_text,
                        });
                    }
                }

                // Handle monster kills: remove from active, start respawn, award XP
                for kill in &result.monster_kills {
                    // Remove dead monster from active_monsters
                    {
                        let mut monsters = active_monsters.write().await;
                        if let Some(room_monsters) = monsters.get_mut(&kill.room_id) {
                            room_monsters.retain(|m| m.id != kill.monster_id);
                        }
                    }

                    // Start respawn timer
                    // Find the spawn entry for this template in the zone data to get respawn_secs
                    let respawn_secs = 120; // Default; TODO: look up from zone spawn data
                    {
                        let mut timers = respawn_timers.write().await;
                        timers.push(RespawnTimer {
                            template_id: kill.template_id.clone(),
                            room_id: kill.room_id.clone(),
                            respawn_at: tokio::time::Instant::now()
                                + Duration::from_secs(respawn_secs),
                        });
                    }

                    // Award XP to participants (split equally)
                    let xp_each = if kill.participants.is_empty() {
                        0
                    } else {
                        kill.xp_value / kill.participants.len() as i32
                    };

                    for char_id in &kill.participants {
                        if let Err(e) = sqlx::query(
                            "UPDATE characters SET xp = xp + ? WHERE id = ?"
                        )
                        .bind(xp_each)
                        .bind(char_id)
                        .execute(&db)
                        .await
                        {
                            warn!(error = %e, "failed to award XP");
                        }
                    }
                }

                // Handle player deaths
                for death in &result.deaths {
                    handle_player_death(
                        &death.character_id,
                        &death.character_name,
                        &world,
                        &room_channels,
                        &db,
                    )
                    .await;
                }

                // Broadcast combat end
                if result.combat_ended {
                    if let Some(ref msg) = result.end_message {
                        let channels = room_channels.read().await;
                        if let Some(sender) = channels.get(room_id) {
                            let _ = sender.send(WorldEvent {
                                message: msg.clone(),
                            });
                        }
                    }
                }
            }
        }

        // 5. Process respawn timers
        process_respawns(
            &respawn_timers,
            &active_monsters,
            &monster_templates,
        )
        .await;
    }
}

/// Snapshot player combat stats from the DB for all players currently in combat.
async fn snapshot_player_stats(
    combat_manager: &Arc<RwLock<CombatManager>>,
    db: &SqlitePool,
) -> HashMap<String, PlayerCombatStats> {
    let player_ids: Vec<String> = {
        let mgr = combat_manager.read().await;
        mgr.combats
            .values()
            .flat_map(|c| c.player_participants.clone())
            .collect()
    };

    let mut stats = HashMap::new();
    for char_id in &player_ids {
        let row: Option<(String, i32, i32, i32, i32, i32, i32, i32, i64, i32, i32, i32, i32, i32, i32, String)> =
            sqlx::query_as(
                "SELECT name, hp, max_hp, mana, max_mana, stamina, max_stamina, xp, level, str_score, dex_score, con_score, int_score, wis_score, cha_score, class FROM characters WHERE id = ?"
            )
            .bind(char_id)
            .fetch_optional(db)
            .await
            .unwrap_or(None);

        if let Some((name, hp, max_hp, mana, max_mana, stamina, max_stamina, xp, level, str_s, dex_s, con_s, int_s, wis_s, cha_s, class)) = row {
            debug!(%char_id, %name, hp, max_hp, "snapshot player stats for combat");
            let pcs = build_player_combat_stats(
                char_id, &name, hp, max_hp, mana, max_mana, stamina, max_stamina,
                xp, level as i32, str_s as u8, dex_s as u8, con_s as u8, int_s as u8, wis_s as u8, cha_s as u8,
                &class,
            );
            stats.insert(char_id.clone(), pcs);
        }
    }
    stats
}

/// Handle a player death: reset HP, teleport to bind point, apply XP debt.
async fn handle_player_death(
    character_id: &str,
    character_name: &str,
    world: &Arc<RwLock<crate::world::types::World>>,
    room_channels: &Arc<RwLock<HashMap<RoomId, broadcast::Sender<WorldEvent>>>>,
    db: &SqlitePool,
) {
    // Get bind point and current room
    let (bind_point, old_room_id) = {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT bind_point FROM characters WHERE id = ?"
        )
        .bind(character_id)
        .fetch_optional(db)
        .await
        .unwrap_or(None);
        let bind = row
            .map(|(bp,)| bp)
            .unwrap_or_else(|| "starting_village:market_square".to_string());

        let w = world.read().await;
        let old_room = w
            .player_positions
            .get(character_id)
            .cloned()
            .unwrap_or_else(|| RoomId(bind.clone()));
        (bind, old_room)
    };

    let bind_room_id = RoomId(bind_point.clone());

    // Apply XP debt (10% of current XP, min 0)
    if let Err(e) = sqlx::query(
        "UPDATE characters SET hp = max_hp, mana = max_mana, stamina = max_stamina, xp = MAX(0, xp - MAX(1, xp / 10)) WHERE id = ?"
    )
    .bind(character_id)
    .execute(db)
    .await
    {
        warn!(error = %e, "failed to update character on death");
    }

    // Move character to bind point in memory
    {
        let mut w = world.write().await;
        w.player_positions
            .insert(character_id.to_string(), bind_room_id.clone());
    }

    // Persist position
    if let Err(e) = sqlx::query(
        "INSERT OR REPLACE INTO character_positions (character_id, room_id, updated_at) VALUES (?, ?, unixepoch())"
    )
    .bind(character_id)
    .bind(&bind_point)
    .execute(db)
    .await
    {
        warn!(error = %e, "failed to persist death respawn position");
    }

    // Broadcast death to old room
    {
        let channels = room_channels.read().await;
        if let Some(sender) = channels.get(&old_room_id) {
            let _ = sender.send(WorldEvent {
                message: format!("{} has been slain!", character_name),
            });
        }
    }

    debug!(%character_id, %character_name, bind_point = %bind_point, "player died and respawned");
}

/// Check respawn timers and spawn new monsters where timers have expired.
async fn process_respawns(
    respawn_timers: &Arc<RwLock<Vec<RespawnTimer>>>,
    active_monsters: &Arc<RwLock<HashMap<RoomId, Vec<ActiveMonster>>>>,
    monster_templates: &Arc<HashMap<String, MonsterTemplate>>,
) {
    let now = tokio::time::Instant::now();

    let mut expired = Vec::new();
    {
        let mut timers = respawn_timers.write().await;
        let mut i = 0;
        while i < timers.len() {
            if timers[i].respawn_at <= now {
                expired.push(timers.remove(i));
            } else {
                i += 1;
            }
        }
    }

    for timer in expired {
        if let Some(template) = monster_templates.get(&timer.template_id) {
            let monster = ActiveMonster::from_template(template, &timer.room_id);
            debug!(
                template_id = %timer.template_id,
                room_id = %timer.room_id,
                "monster respawned"
            );
            let mut monsters = active_monsters.write().await;
            monsters
                .entry(timer.room_id)
                .or_default()
                .push(monster);
        }
    }
}

/// Load monster templates from the TOML data file.
pub fn load_monster_templates(
    data_path: &std::path::Path,
) -> anyhow::Result<HashMap<String, MonsterTemplate>> {
    let content = std::fs::read_to_string(data_path)?;
    let data: MonsterDataFile = toml::from_str(&content)?;

    let mut templates = HashMap::new();
    for template in data.monsters {
        templates.insert(template.id.clone(), template);
    }
    Ok(templates)
}

/// Spawn initial monsters based on zone spawn tables.
///
/// `spawn_tables` maps room_id -> vec of (monster_template_id, count).
pub fn spawn_initial_monsters(
    spawn_tables: &HashMap<RoomId, Vec<SpawnEntry>>,
    templates: &HashMap<String, MonsterTemplate>,
) -> HashMap<RoomId, Vec<ActiveMonster>> {
    let mut result: HashMap<RoomId, Vec<ActiveMonster>> = HashMap::new();

    for (room_id, entries) in spawn_tables {
        for entry in entries {
            if let Some(template) = templates.get(&entry.monster) {
                for _ in 0..entry.count {
                    let monster = ActiveMonster::from_template(template, room_id);
                    result.entry(room_id.clone()).or_default().push(monster);
                }
            } else {
                warn!(
                    template_id = %entry.monster,
                    room_id = %room_id,
                    "unknown monster template in spawn table"
                );
            }
        }
    }

    result
}
