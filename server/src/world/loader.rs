// Loader implementation is completed in plan 02-01 Task 2.
// This stub allows the module to compile during Task 1.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;
use tracing::info;

use crate::world::types::{RoomDef, RoomId, RoomState, World};

#[derive(Deserialize)]
struct ZoneFile {
    zone_id: String,
    zone_name: String,
    rooms: Vec<RoomDef>,
}

/// Load all zone TOML files from `zones_dir`, then overlay persisted state from SQLite.
///
/// This runs once at server startup. File I/O is synchronous (acceptable before the
/// async listener starts). SQLite reads are async.
pub async fn load_world(
    zones_dir: &Path,
    pool: &sqlx::SqlitePool,
) -> anyhow::Result<World> {
    let mut rooms = HashMap::new();
    let mut zone_count = 0u32;

    // Recurse one level: zones_dir/zone_name/zone.toml
    for entry in fs::read_dir(zones_dir)? {
        let entry = entry?;
        let zone_path = entry.path().join("zone.toml");
        if zone_path.exists() {
            let content = fs::read_to_string(&zone_path)?;
            let zone: ZoneFile = toml::from_str(&content)
                .map_err(|e| anyhow::anyhow!("failed to parse {:?}: {e}", zone_path))?;
            // Validate no room ID collisions
            for room in &zone.rooms {
                let room_id = RoomId(room.id.clone());
                if rooms.contains_key(&room_id) {
                    return Err(anyhow::anyhow!(
                        "duplicate room ID '{}' found while loading zone '{}'",
                        room.id, zone.zone_id
                    ));
                }
            }
            let zone_id = zone.zone_id.clone();
            let zone_name = zone.zone_name.clone();
            for room in zone.rooms {
                rooms.insert(RoomId(room.id.clone()), room);
            }
            zone_count += 1;
            info!(zone_id = %zone_id, zone_name = %zone_name, "loaded zone");
        }
    }

    info!(zone_count, room_count = rooms.len(), "world loaded from TOML");

    // Overlay persisted trigger state from SQLite
    let state_rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT room_id, state_key, state_value FROM world_state"
    )
    .fetch_all(pool)
    .await?;

    let mut room_states: HashMap<RoomId, RoomState> = HashMap::new();
    for (room_id, key, value) in state_rows {
        let state = room_states.entry(RoomId(room_id)).or_default();
        state.kv.insert(key, value);
    }

    // Load persisted player positions
    let position_rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT account_id, room_id FROM player_positions"
    )
    .fetch_all(pool)
    .await?;
    let player_positions: HashMap<String, RoomId> = position_rows
        .into_iter()
        .map(|(account_id, room_id)| (account_id, RoomId(room_id)))
        .collect();

    info!(
        persisted_states = room_states.len(),
        persisted_positions = player_positions.len(),
        "overlaid persisted state from SQLite"
    );

    Ok(World { rooms, room_states, player_positions, default_spawn: None })
}
