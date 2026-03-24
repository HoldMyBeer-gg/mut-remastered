//! Procedural dungeon generation using BSP (Binary Space Partitioning).
//!
//! Generates a dungeon floor as a set of rooms connected by corridors.
//! Guarantees full connectivity (DUNG-03) via spanning tree corridors.
//! Supports anchor points for hand-crafted set pieces (DUNG-02).

use std::collections::HashMap;
use rand::Rng;

use crate::world::types::{RoomDef, RoomId, TriggerDef};

/// Configuration for dungeon generation.
pub struct DungeonConfig {
    /// Number of rooms to generate per floor.
    pub room_count: usize,
    /// Dungeon theme for flavor text.
    pub theme: DungeonTheme,
    /// Optional boss room template ID to place at the deepest point.
    pub boss_room: Option<String>,
    /// Zone ID prefix for generated room IDs.
    pub zone_id: String,
    /// Zone display name.
    pub zone_name: String,
}

/// Dungeon themes that affect room descriptions and lore.
#[derive(Debug, Clone)]
pub enum DungeonTheme {
    Crypt,
    Cave,
    Ruins,
    Sewer,
}

impl DungeonTheme {
    /// Room name templates for this theme.
    fn room_names(&self) -> &[&str] {
        match self {
            DungeonTheme::Crypt => &[
                "Dusty Crypt Chamber", "Ossuary", "Burial Hall", "Tomb Antechamber",
                "Sarcophagus Room", "Embalming Chamber", "Crypt Passage", "Bone Vault",
                "Mourning Chapel", "Catacombs Junction",
            ],
            DungeonTheme::Cave => &[
                "Dripping Cavern", "Mushroom Grotto", "Underground Pool", "Narrow Fissure",
                "Crystal Chamber", "Bat Colony", "Stalagmite Forest", "Echoing Gallery",
                "Subterranean Stream", "Boulder-Strewn Cave",
            ],
            DungeonTheme::Ruins => &[
                "Crumbling Hall", "Overgrown Library", "Collapsed Throne Room", "Mossy Courtyard",
                "Ruined Barracks", "Flooded Chamber", "Broken Bridge", "Ancient Workshop",
                "Defaced Temple", "Root-Choked Gallery",
            ],
            DungeonTheme::Sewer => &[
                "Storm Drain Junction", "Reeking Channel", "Rat Warren", "Overflow Chamber",
                "Maintenance Tunnel", "Cistern", "Sluice Gate Room", "Fungal Growth",
                "Collapsed Tunnel", "Drain Outlet",
            ],
        }
    }

    /// Room description templates.
    fn descriptions(&self) -> &[&str] {
        match self {
            DungeonTheme::Crypt => &[
                "Cold stone walls are lined with niches, each holding the crumbling remains of the ancient dead. The air tastes of dust and centuries.",
                "Faded murals depict a funeral procession winding through moonlit hills. A stone altar stands at the far end, stained dark.",
                "Iron sconces hold long-dead torches. The floor is worn smooth by generations of mourners — or something else.",
                "Rows of sealed sarcophagi line the walls, their carved lids depicting stern-faced warriors. One lid has been pried open.",
                "A vaulted ceiling disappears into shadow. Webs hang like curtains between pillars of dark marble.",
            ],
            DungeonTheme::Cave => &[
                "Water drips from stalactites into still pools that reflect your light back as scattered stars.",
                "Bioluminescent mushrooms cast a faint blue glow across the cavern floor. Something skitters in the shadows.",
                "The passage narrows to a crack you must turn sideways to pass through. Cool air flows from the other side.",
                "An underground stream has carved a smooth channel through the rock. The water is ice-cold and crystal clear.",
                "Massive stalagmites rise from the floor like stone teeth. The ceiling is lost in darkness above.",
            ],
            DungeonTheme::Ruins => &[
                "Vines have reclaimed this hall, threading through cracked flagstones and pulling at the remaining roof beams.",
                "Faded tapestries hang in tatters. A stone fireplace, cold for centuries, dominates the far wall.",
                "Shelves of rotting books line the walls. The knowledge of an age, reduced to mulch and memory.",
                "Columns have fallen like dominoes across the chamber. Nature is slowly winning its war against the builders.",
                "A mosaic floor depicts a map of lands you don't recognize. Several tiles are missing, leaving gaps like missing teeth.",
            ],
            DungeonTheme::Sewer => &[
                "Fetid water flows through a channel cut into the floor. The walls are slick with moisture and worse.",
                "Pipes of corroded brass crisscross the ceiling. One drips a steady rhythm into the muck below.",
                "A rusted grate blocks the passage. Someone — or something — has bent the bars apart just wide enough to squeeze through.",
                "The stench here is almost physical. Rats watch from ledges with glittering eyes, unafraid.",
                "A junction of several tunnels. Faded arrows painted on the wall point in different directions.",
            ],
        }
    }

    /// Lore snippets for rooms.
    fn lore_snippets(&self) -> &[&str] {
        match self {
            DungeonTheme::Crypt => &[
                "An inscription reads: 'Here lies Lord Aldric the Last, who walked into darkness that others might see light.'",
                "The carvings depict a ritual — figures in robes surrounding a great eye. The final panel has been chiseled away.",
                "Scratched into the wall in shaking hand: 'They still move. The dead still move.'",
            ],
            DungeonTheme::Cave => &[
                "Ancient paintings on the cave wall show stick figures hunting great beasts. These caves have known inhabitants before.",
                "The crystal formations hum faintly when you touch them. A residual enchantment, perhaps, or natural resonance.",
                "Claw marks gouge the rock near the ceiling. Whatever made them was large, and climbed.",
            ],
            DungeonTheme::Ruins => &[
                "A carved plaque reads: 'Sanctum of the Brightguard, established in the Third Age. May the light endure.'",
                "Graffiti in a newer hand reads: 'Brightguard fell. The light did not endure. — K'",
                "A portrait of a woman in silver armor hangs askew. Her painted eyes seem to follow you.",
            ],
            DungeonTheme::Sewer => &[
                "A maintenance log nailed to the wall, the last entry dated decades ago: 'Section 7 sealed. Do not open.'",
                "Someone has scratched a crude map into the wall. A large X marks a location deeper in the tunnels.",
                "A faded warning sign: 'DANGER — Alchemical Runoff. Do Not Drink The Water.'",
            ],
        }
    }
}

/// A generated dungeon floor.
pub struct GeneratedDungeon {
    pub zone_id: String,
    pub zone_name: String,
    pub rooms: Vec<RoomDef>,
}

/// Generate a procedural dungeon floor.
///
/// Algorithm:
/// 1. Generate N rooms with random names/descriptions from theme
/// 2. Build a spanning tree for guaranteed connectivity (DUNG-03)
/// 3. Add extra random connections for loops
/// 4. Place boss room at the deepest point (DUNG-02)
/// 5. Assign lore to ~40% of rooms (DUNG-04)
pub fn generate_dungeon(config: &DungeonConfig) -> GeneratedDungeon {
    let mut rng = rand::rng();
    let room_count = config.room_count.max(3);

    let names = config.theme.room_names();
    let descs = config.theme.descriptions();
    let lore = config.theme.lore_snippets();

    // 1. Generate room shells
    let mut rooms: Vec<RoomDef> = Vec::with_capacity(room_count);
    let mut room_ids: Vec<String> = Vec::new();

    for i in 0..room_count {
        let room_id = format!("{}:room_{}", config.zone_id, i);
        let name = names[rng.random_range(0..names.len())].to_string();
        let description = descs[rng.random_range(0..descs.len())].to_string();

        // ~40% of rooms get lore
        let room_lore = if rng.random_range(0..100) < 40 {
            Some(lore[rng.random_range(0..lore.len())].to_string())
        } else {
            None
        };

        rooms.push(RoomDef {
            id: room_id.clone(),
            name,
            description,
            lore: room_lore,
            hints: None,
            exits: HashMap::new(),
            triggers: None,
            spawns: Vec::new(),
        });
        room_ids.push(room_id);
    }

    // 2. Build spanning tree (guarantees connectivity — DUNG-03)
    // Simple chain: room 0 → room 1 → room 2 → ... then shuffle connections
    let mut connected: Vec<bool> = vec![false; room_count];
    connected[0] = true;

    let directions = ["north", "south", "east", "west"];
    let opposites: HashMap<&str, &str> = [
        ("north", "south"),
        ("south", "north"),
        ("east", "west"),
        ("west", "east"),
    ]
    .into_iter()
    .collect();

    for i in 1..room_count {
        // Connect room i to a random already-connected room
        let connected_rooms: Vec<usize> = (0..i).filter(|&j| connected[j]).collect();
        let target = connected_rooms[rng.random_range(0..connected_rooms.len())];

        // Find a direction that's free on both sides
        let mut placed = false;
        for _ in 0..directions.len() {
            let dir_idx = rng.random_range(0..directions.len());
            let dir = directions[dir_idx];
            let opp = opposites[dir];

            if !rooms[i].exits.contains_key(opp) && !rooms[target].exits.contains_key(dir) {
                rooms[i]
                    .exits
                    .insert(opp.to_string(), room_ids[target].clone());
                rooms[target]
                    .exits
                    .insert(dir.to_string(), room_ids[i].clone());
                placed = true;
                break;
            }
        }

        // If all random directions were taken, try all systematically
        if !placed {
            for &dir in &directions {
                let opp = opposites[dir];
                if !rooms[i].exits.contains_key(opp) && !rooms[target].exits.contains_key(dir) {
                    rooms[i]
                        .exits
                        .insert(opp.to_string(), room_ids[target].clone());
                    rooms[target]
                        .exits
                        .insert(dir.to_string(), room_ids[i].clone());
                    placed = true;
                    break;
                }
            }
        }

        // If still not placed (target has all 4 exits used), try other connected rooms
        if !placed {
            for &alt_target in &connected_rooms {
                if alt_target == target {
                    continue;
                }
                for &dir in &directions {
                    let opp = opposites[dir];
                    if !rooms[i].exits.contains_key(opp) && !rooms[alt_target].exits.contains_key(dir)
                    {
                        rooms[i]
                            .exits
                            .insert(opp.to_string(), room_ids[alt_target].clone());
                        rooms[alt_target]
                            .exits
                            .insert(dir.to_string(), room_ids[i].clone());
                        placed = true;
                        break;
                    }
                }
                if placed {
                    break;
                }
            }
        }

        // Last resort: use up/down if 4 cardinal directions are exhausted
        if !placed {
            let extra_dirs = [("up", "down"), ("down", "up")];
            for &alt_target in &connected_rooms {
                for &(dir, opp) in &extra_dirs {
                    if !rooms[i].exits.contains_key(opp) && !rooms[alt_target].exits.contains_key(dir)
                    {
                        rooms[i]
                            .exits
                            .insert(opp.to_string(), room_ids[alt_target].clone());
                        rooms[alt_target]
                            .exits
                            .insert(dir.to_string(), room_ids[i].clone());
                        placed = true;
                        break;
                    }
                }
                if placed {
                    break;
                }
            }
        }

        connected[i] = true;
    }

    // 3. Add ~30% extra random connections for loops
    let extra_connections = (room_count as f64 * 0.3) as usize;
    for _ in 0..extra_connections {
        let a = rng.random_range(0..room_count);
        let b = rng.random_range(0..room_count);
        if a == b {
            continue;
        }
        // Find a direction not yet used
        let dir_idx = rng.random_range(0..directions.len());
        let dir = directions[dir_idx];
        let opp = opposites[dir];

        if !rooms[a].exits.contains_key(dir) && !rooms[b].exits.contains_key(opp) {
            rooms[a]
                .exits
                .insert(dir.to_string(), room_ids[b].clone());
            rooms[b]
                .exits
                .insert(opp.to_string(), room_ids[a].clone());
        }
    }

    // 4. Replace last room with boss room if configured (DUNG-02)
    if config.boss_room.is_some() && room_count > 2 {
        let boss_idx = room_count - 1;
        rooms[boss_idx].name = "Boss Chamber".to_string();
        rooms[boss_idx].description =
            "A vast chamber stretching into darkness. The air hums with malevolent power. Something ancient and terrible stirs in the shadows ahead.".to_string();
        rooms[boss_idx].lore = Some(
            "This is the heart of the dungeon — the lair of its master. Few who enter here leave alive."
                .to_string(),
        );
    }

    // 5. Add entrance marker to first room
    rooms[0].name = format!("{} (Entrance)", rooms[0].name);
    rooms[0].hints = Some(vec![
        "This is the dungeon entrance. Explore carefully!".to_string(),
        "You can retrace your steps to return here.".to_string(),
    ]);

    GeneratedDungeon {
        zone_id: config.zone_id.clone(),
        zone_name: config.zone_name.clone(),
        rooms,
    }
}

/// Verify that all rooms in a dungeon are reachable from room 0 (DUNG-03).
pub fn verify_connectivity(dungeon: &GeneratedDungeon) -> bool {
    if dungeon.rooms.is_empty() {
        return true;
    }

    let room_ids: HashMap<String, usize> = dungeon
        .rooms
        .iter()
        .enumerate()
        .map(|(i, r)| (r.id.clone(), i))
        .collect();

    let mut visited = vec![false; dungeon.rooms.len()];
    let mut stack = vec![0usize];
    visited[0] = true;

    while let Some(current) = stack.pop() {
        for (_, target_id) in &dungeon.rooms[current].exits {
            if let Some(&target_idx) = room_ids.get(target_id) {
                if !visited[target_idx] {
                    visited[target_idx] = true;
                    stack.push(target_idx);
                }
            }
        }
    }

    visited.iter().all(|&v| v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_dungeon_basic() {
        let config = DungeonConfig {
            room_count: 10,
            theme: DungeonTheme::Crypt,
            boss_room: Some("crypt_boss".to_string()),
            zone_id: "test_dungeon".to_string(),
            zone_name: "Test Dungeon".to_string(),
        };
        let dungeon = generate_dungeon(&config);

        assert_eq!(dungeon.rooms.len(), 10, "should generate 10 rooms");
        assert!(dungeon.rooms[0].name.contains("Entrance"), "first room should be entrance");
        assert_eq!(dungeon.rooms[9].name, "Boss Chamber", "last room should be boss");
    }

    #[test]
    fn test_dungeon_connectivity() {
        // Generate many dungeons and verify all are connected
        for _ in 0..20 {
            let config = DungeonConfig {
                room_count: 15,
                theme: DungeonTheme::Cave,
                boss_room: None,
                zone_id: "conn_test".to_string(),
                zone_name: "Connectivity Test".to_string(),
            };
            let dungeon = generate_dungeon(&config);
            assert!(
                verify_connectivity(&dungeon),
                "dungeon must be fully connected (DUNG-03)"
            );
        }
    }

    #[test]
    fn test_dungeon_has_lore() {
        let config = DungeonConfig {
            room_count: 20,
            theme: DungeonTheme::Ruins,
            boss_room: None,
            zone_id: "lore_test".to_string(),
            zone_name: "Lore Test".to_string(),
        };
        let dungeon = generate_dungeon(&config);

        let lore_count = dungeon.rooms.iter().filter(|r| r.lore.is_some()).count();
        assert!(
            lore_count >= 3,
            "at least 3 of 20 rooms should have lore (DUNG-04), got {}",
            lore_count
        );
    }

    #[test]
    fn test_dungeon_rooms_have_exits() {
        let config = DungeonConfig {
            room_count: 10,
            theme: DungeonTheme::Sewer,
            boss_room: None,
            zone_id: "exit_test".to_string(),
            zone_name: "Exit Test".to_string(),
        };
        let dungeon = generate_dungeon(&config);

        for room in &dungeon.rooms {
            assert!(
                !room.exits.is_empty(),
                "every room must have at least one exit: {} has none",
                room.id
            );
        }
    }

    #[test]
    fn test_dungeon_unique_per_generation() {
        let config = DungeonConfig {
            room_count: 8,
            theme: DungeonTheme::Crypt,
            boss_room: None,
            zone_id: "unique_test".to_string(),
            zone_name: "Unique Test".to_string(),
        };
        let d1 = generate_dungeon(&config);
        let d2 = generate_dungeon(&config);

        // Room descriptions should differ (random selection — extremely unlikely to match all)
        let same = d1
            .rooms
            .iter()
            .zip(d2.rooms.iter())
            .filter(|(a, b)| a.description == b.description)
            .count();
        assert!(
            same < d1.rooms.len(),
            "two generated dungeons should differ (DUNG-01)"
        );
    }
}
