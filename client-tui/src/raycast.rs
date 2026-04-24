//! Wolfenstein-style raycasting renderer for terminal.
//!
//! Casts rays across a 2D tile grid to produce a first-person 3D perspective
//! using Unicode block characters for wall shading by distance.
//! The grid is built from the current room's exits and neighboring rooms.

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::GameState;

/// Tile types in the local map grid.
#[derive(Clone, Copy, PartialEq)]
enum Tile {
    Wall,
    Floor,
    Door,
}

const GRID_SIZE: usize = 17; // Must be odd so player is centered
const HALF: usize = GRID_SIZE / 2;

/// Build a local tile grid from the player's current room exits.
/// Player is at the center. Exits create openings in walls.
fn build_local_grid(exits: &[String]) -> [[Tile; GRID_SIZE]; GRID_SIZE] {
    let mut grid = [[Tile::Wall; GRID_SIZE]; GRID_SIZE];

    // Carve out the center room (5x5 floor area)
    for y in (HALF - 2)..=(HALF + 2) {
        for x in (HALF - 2)..=(HALF + 2) {
            grid[y][x] = Tile::Floor;
        }
    }

    // Carve corridors for each exit
    for exit in exits {
        match exit.as_str() {
            "north" => {
                for y in 0..=(HALF - 2) {
                    grid[y][HALF - 1] = Tile::Floor;
                    grid[y][HALF] = Tile::Floor;
                    grid[y][HALF + 1] = Tile::Floor;
                }
                grid[HALF - 3][HALF] = Tile::Door;
            }
            "south" => {
                for y in (HALF + 2)..GRID_SIZE {
                    grid[y][HALF - 1] = Tile::Floor;
                    grid[y][HALF] = Tile::Floor;
                    grid[y][HALF + 1] = Tile::Floor;
                }
                grid[HALF + 3][HALF] = Tile::Door;
            }
            "east" => {
                for x in (HALF + 2)..GRID_SIZE {
                    grid[HALF - 1][x] = Tile::Floor;
                    grid[HALF][x] = Tile::Floor;
                    grid[HALF + 1][x] = Tile::Floor;
                }
                grid[HALF][HALF + 3] = Tile::Door;
            }
            "west" => {
                for x in 0..=(HALF - 2) {
                    grid[HALF - 1][x] = Tile::Floor;
                    grid[HALF][x] = Tile::Floor;
                    grid[HALF + 1][x] = Tile::Floor;
                }
                grid[HALF][HALF - 3] = Tile::Door;
            }
            _ => {}
        }
    }

    grid
}

/// Cast rays and render a first-person view into the given Ratatui area.
pub fn render_raycast_view(f: &mut Frame, state: &GameState, area: Rect) {
    let block = Block::bordered()
        .title(format!(" {} ", state.room_name))
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let screen_w = inner.width as usize;
    let screen_h = inner.height as usize;
    if screen_w < 4 || screen_h < 4 {
        return;
    }

    let grid = build_local_grid(&state.room_exits);

    // Player position and facing direction
    // During walk animation, offset position forward to simulate movement
    let walk_progress = state.camera_walk;
    let walk_offset = if state.camera_animating {
        // Ease-out: fast at start, slow at end
        let t = 1.0 - (1.0 - walk_progress).powi(3);
        t * 3.0 // Move forward up to 3 tiles during transition
    } else {
        0.0
    };

    // Head bob during walking
    let time = state.frame as f64 * 0.1;
    let head_bob = if state.camera_animating {
        (time * 8.0).sin() * 0.15 * (1.0 - walk_progress) // Bob fades as you settle
    } else {
        (time * 1.5).sin() * 0.02 // Very subtle idle breathing
    };

    let player_x: f64 = HALF as f64 + 0.5;
    let player_y: f64 = HALF as f64 + 0.5 - walk_offset;
    // Face north by default (negative Y), with slight sway
    let sway = if state.camera_animating {
        (time * 6.0).sin() * 0.03 * (1.0 - walk_progress)
    } else {
        0.0
    };
    let dir_x: f64 = sway;
    let dir_y: f64 = -1.0;
    // Camera plane (perpendicular to direction)
    let plane_x: f64 = 0.66;
    let plane_y: f64 = sway * 0.3;

    // Character buffer
    let mut buf: Vec<Vec<(char, Color, Color)>> =
        vec![vec![(' ', Color::Reset, Color::Reset); screen_w]; screen_h];

    // Draw ceiling and floor background with head bob offset
    let horizon = (screen_h as f64 / 2.0 + head_bob * screen_h as f64) as usize;
    for y in 0..screen_h {
        for x in 0..screen_w {
            if y < horizon {
                // Ceiling — dark gradient
                let brightness = 15 + (y as u8 * 2).min(30);
                buf[y][x] = (
                    ' ',
                    Color::Reset,
                    Color::Rgb(brightness, brightness, brightness + 10),
                );
            } else {
                // Floor — brown gradient
                let dist = (y - horizon) as u8;
                let brightness = 25 + dist.min(40);
                buf[y][x] = (
                    ' ',
                    Color::Reset,
                    Color::Rgb(brightness, brightness / 2 + 10, brightness / 3),
                );
            }
        }
    }

    // Cast one ray per screen column
    for x in 0..screen_w {
        let camera_x = 2.0 * x as f64 / screen_w as f64 - 1.0;
        let ray_dir_x = dir_x + plane_x * camera_x;
        let ray_dir_y = dir_y + plane_y * camera_x;

        // DDA raycasting
        let mut map_x = player_x as i32;
        let mut map_y = player_y as i32;

        let delta_dist_x = if ray_dir_x == 0.0 {
            f64::MAX
        } else {
            (1.0 / ray_dir_x).abs()
        };
        let delta_dist_y = if ray_dir_y == 0.0 {
            f64::MAX
        } else {
            (1.0 / ray_dir_y).abs()
        };

        let step_x: i32;
        let step_y: i32;
        let mut side_dist_x: f64;
        let mut side_dist_y: f64;

        if ray_dir_x < 0.0 {
            step_x = -1;
            side_dist_x = (player_x - map_x as f64) * delta_dist_x;
        } else {
            step_x = 1;
            side_dist_x = (map_x as f64 + 1.0 - player_x) * delta_dist_x;
        }
        if ray_dir_y < 0.0 {
            step_y = -1;
            side_dist_y = (player_y - map_y as f64) * delta_dist_y;
        } else {
            step_y = 1;
            side_dist_y = (map_y as f64 + 1.0 - player_y) * delta_dist_y;
        }

        // Perform DDA
        let mut hit = false;
        let mut side = 0; // 0 = NS wall, 1 = EW wall
        let mut hit_tile = Tile::Wall;

        for _ in 0..20 {
            if side_dist_x < side_dist_y {
                side_dist_x += delta_dist_x;
                map_x += step_x;
                side = 0;
            } else {
                side_dist_y += delta_dist_y;
                map_y += step_y;
                side = 1;
            }

            if map_x < 0 || map_y < 0 || map_x >= GRID_SIZE as i32 || map_y >= GRID_SIZE as i32 {
                hit = true;
                hit_tile = Tile::Wall;
                break;
            }

            let tile = grid[map_y as usize][map_x as usize];
            if tile == Tile::Wall {
                hit = true;
                hit_tile = Tile::Wall;
                break;
            }
        }

        if !hit {
            continue;
        }

        // Calculate wall distance (perpendicular to avoid fisheye)
        let perp_wall_dist = if side == 0 {
            (map_x as f64 - player_x + (1.0 - step_x as f64) / 2.0) / ray_dir_x
        } else {
            (map_y as f64 - player_y + (1.0 - step_y as f64) / 2.0) / ray_dir_y
        };

        let perp_wall_dist = perp_wall_dist.max(0.1);

        // Calculate wall column height
        let line_height = (screen_h as f64 / perp_wall_dist) as i32;
        let horizon_i32 = horizon as i32;
        let draw_start = (-(line_height) / 2 + horizon_i32).max(0) as usize;
        let draw_end = ((line_height) / 2 + horizon_i32).min(screen_h as i32 - 1) as usize;

        // Choose wall character and color based on distance and side
        let (ch, color) = wall_shade(perp_wall_dist, side);

        for y in draw_start..=draw_end {
            if y < screen_h {
                buf[y][x] = (ch, color, Color::Reset);
            }
        }
    }

    // Draw 3D mesh objects — throttled to every 4th frame to keep the loop responsive
    if !state.monsters_here.is_empty() && state.frame.is_multiple_of(4) {
        use crate::mesh3d::*;

        let time = state.frame as f64 * 0.05;
        let mut mesh_buf: Vec<Vec<(char, [u8; 3])>> =
            vec![vec![(' ', [0, 0, 0]); screen_w]; screen_h];
        let mut mesh_zbuf: Vec<Vec<f64>> = vec![vec![f64::MAX; screen_w]; screen_h];

        // Render each monster with appropriate shape and color
        let count = state.monsters_here.len();
        for (i, monster_name) in state.monsters_here.iter().enumerate() {
            let name_lower = monster_name.to_lowercase();

            // Pick mesh and color based on monster name
            let (mesh, color, scale) =
                if name_lower.contains("skeleton") || name_lower.contains("boss") {
                    (create_skull(), [200u8, 200, 180], 1.2)
                } else if name_lower.contains("spider") {
                    (create_diamond(), [100u8, 60, 120], 0.8) // purple-ish
                } else if name_lower.contains("goblin") {
                    (create_monster(), [80u8, 180, 80], 0.9) // green
                } else if name_lower.contains("rat") {
                    (create_cube(), [160u8, 140, 100], 0.5) // small brown
                } else {
                    (create_monster(), [220u8, 80, 80], 1.0) // default red humanoid
                };

            // Offset each monster so multiple don't overlap
            let x_offset = if count > 1 {
                (i as f64 - (count as f64 - 1.0) / 2.0) * 2.5
            } else {
                0.0
            };

            render_mesh(
                &mut mesh_buf,
                &mut mesh_zbuf,
                &mesh,
                (time * 0.2, time * 0.5, 0.0),
                Vec3::new(x_offset, -0.2, 5.0),
                scale,
                color,
            );
        }

        // Composite onto raycasted scene
        for y in 0..screen_h {
            for x in 0..screen_w {
                if mesh_buf[y][x].0 != ' ' {
                    let [r, g, b] = mesh_buf[y][x].1;
                    buf[y][x] = (mesh_buf[y][x].0, Color::Rgb(r, g, b), Color::Reset);
                }
            }
        }
    }

    // Draw exit labels
    let mid_x = screen_w / 2;
    let mid_y = screen_h / 2;

    for exit in &state.room_exits {
        match exit.as_str() {
            "north" => {
                if mid_y > 2 && mid_x < screen_w {
                    buf[1][mid_x] = ('N', Color::Yellow, Color::Reset);
                }
            }
            "south" => {
                if screen_h > 2 && mid_x < screen_w {
                    buf[screen_h - 2][mid_x] = ('S', Color::Rgb(120, 120, 60), Color::Reset);
                }
            }
            "east" => {
                if screen_w > 2 && mid_y < screen_h {
                    buf[mid_y][screen_w - 2] = ('E', Color::Yellow, Color::Reset);
                }
            }
            "west" => {
                if mid_y < screen_h {
                    buf[mid_y][1] = ('W', Color::Yellow, Color::Reset);
                }
            }
            _ => {}
        }
    }

    // Render buffer
    for (row_idx, row) in buf.iter().enumerate() {
        if row_idx >= inner.height as usize {
            break;
        }
        let spans: Vec<Span> = row
            .iter()
            .take(inner.width as usize)
            .map(|&(ch, fg, bg)| {
                let mut style = Style::default();
                if fg != Color::Reset {
                    style = style.fg(fg);
                }
                if bg != Color::Reset {
                    style = style.bg(bg);
                }
                Span::styled(ch.to_string(), style)
            })
            .collect();
        let line = Line::from(spans);
        let para = Paragraph::new(line);
        let row_area = Rect {
            x: inner.x,
            y: inner.y + row_idx as u16,
            width: inner.width,
            height: 1,
        };
        f.render_widget(para, row_area);
    }
}

/// Choose wall character and color based on perpendicular distance.
fn wall_shade(dist: f64, side: i32) -> (char, Color) {
    // Side 1 (EW walls) are slightly darker for depth perception
    let darken = if side == 1 { 30u8 } else { 0u8 };

    if dist < 1.5 {
        (
            '█',
            Color::Rgb(
                180u8.saturating_sub(darken),
                170u8.saturating_sub(darken),
                160u8.saturating_sub(darken),
            ),
        )
    } else if dist < 2.5 {
        (
            '▓',
            Color::Rgb(
                140u8.saturating_sub(darken),
                130u8.saturating_sub(darken),
                120u8.saturating_sub(darken),
            ),
        )
    } else if dist < 4.0 {
        (
            '▒',
            Color::Rgb(
                100u8.saturating_sub(darken),
                90u8.saturating_sub(darken),
                80u8.saturating_sub(darken),
            ),
        )
    } else if dist < 6.0 {
        (
            '░',
            Color::Rgb(
                70u8.saturating_sub(darken),
                60u8.saturating_sub(darken),
                50u8.saturating_sub(darken),
            ),
        )
    } else {
        (
            '·',
            Color::Rgb(
                40u8.saturating_sub(darken),
                35u8.saturating_sub(darken),
                30u8.saturating_sub(darken),
            ),
        )
    }
}
