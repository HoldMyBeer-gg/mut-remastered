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
    let player_x: f64 = HALF as f64 + 0.5;
    let player_y: f64 = HALF as f64 + 0.5;
    // Face north by default (negative Y)
    let dir_x: f64 = 0.0;
    let dir_y: f64 = -1.0;
    // Camera plane (perpendicular to direction)
    let plane_x: f64 = 0.66;
    let plane_y: f64 = 0.0;

    // Character buffer
    let mut buf: Vec<Vec<(char, Color, Color)>> =
        vec![vec![(' ', Color::Reset, Color::Reset); screen_w]; screen_h];

    // Draw ceiling and floor background
    for y in 0..screen_h {
        for x in 0..screen_w {
            if y < screen_h / 2 {
                // Ceiling — dark gradient
                let brightness = 15 + (y as u8 * 2).min(30);
                buf[y][x] = (' ', Color::Reset, Color::Rgb(brightness, brightness, brightness + 10));
            } else {
                // Floor — brown gradient
                let dist = (y - screen_h / 2) as u8;
                let brightness = 25 + dist.min(40);
                buf[y][x] = (' ', Color::Reset, Color::Rgb(brightness, brightness / 2 + 10, brightness / 3));
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

        let delta_dist_x = if ray_dir_x == 0.0 { f64::MAX } else { (1.0 / ray_dir_x).abs() };
        let delta_dist_y = if ray_dir_y == 0.0 { f64::MAX } else { (1.0 / ray_dir_y).abs() };

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
        let draw_start = (-(line_height) / 2 + screen_h as i32 / 2).max(0) as usize;
        let draw_end = ((line_height) / 2 + screen_h as i32 / 2).min(screen_h as i32 - 1) as usize;

        // Choose wall character and color based on distance and side
        let (ch, color) = wall_shade(perp_wall_dist, side);

        for y in draw_start..=draw_end {
            if y < screen_h {
                buf[y][x] = (ch, color, Color::Reset);
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
                if fg != Color::Reset { style = style.fg(fg); }
                if bg != Color::Reset { style = style.bg(bg); }
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
        ('█', Color::Rgb(180u8.saturating_sub(darken), 170u8.saturating_sub(darken), 160u8.saturating_sub(darken)))
    } else if dist < 2.5 {
        ('▓', Color::Rgb(140u8.saturating_sub(darken), 130u8.saturating_sub(darken), 120u8.saturating_sub(darken)))
    } else if dist < 4.0 {
        ('▒', Color::Rgb(100u8.saturating_sub(darken), 90u8.saturating_sub(darken), 80u8.saturating_sub(darken)))
    } else if dist < 6.0 {
        ('░', Color::Rgb(70u8.saturating_sub(darken), 60u8.saturating_sub(darken), 50u8.saturating_sub(darken)))
    } else {
        ('·', Color::Rgb(40u8.saturating_sub(darken), 35u8.saturating_sub(darken), 30u8.saturating_sub(darken)))
    }
}
