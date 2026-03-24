//! First-person raycasting dungeon view rendered in ASCII.
//!
//! Renders a Wolfenstein 3D-style view using Unicode block characters
//! for depth shading. The dungeon is a simple grid where exits define
//! openings in walls.

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::GameState;

/// Render the first-person dungeon view.
pub fn render_dungeon_view(f: &mut Frame, state: &GameState, area: Rect) {
    let block = Block::bordered()
        .title(" Dungeon View ")
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width < 4 || inner.height < 4 {
        return;
    }

    let w = inner.width as usize;
    let h = inner.height as usize;

    // Build the scene based on current room exits
    let exits = &state.room_exits;
    let has_north = exits.iter().any(|e| e == "north");
    let has_south = exits.iter().any(|e| e == "south");
    let has_east = exits.iter().any(|e| e == "east");
    let has_west = exits.iter().any(|e| e == "west");

    // Create character buffer
    let mut buf: Vec<Vec<(char, Color)>> = vec![vec![(' ', Color::Black); w]; h];

    // Render the first-person perspective
    // The view shows a corridor looking "north" (forward)
    // with openings on left (west), right (east), and ahead (north)

    let mid_x = w / 2;
    let mid_y = h / 2;

    // Draw ceiling and floor gradient
    for y in 0..h {
        for x in 0..w {
            let dist_from_center = ((y as f64 - mid_y as f64).abs() / mid_y as f64).min(1.0);
            if y < mid_y {
                // Ceiling
                let shade = ceiling_shade(dist_from_center);
                buf[y][x] = (shade, Color::Rgb(40, 40, 60));
            } else {
                // Floor
                let shade = floor_shade(dist_from_center);
                buf[y][x] = (shade, Color::Rgb(60, 50, 40));
            }
        }
    }

    // Draw walls with perspective
    // Wall depth layers from far to near
    let layers = [
        (0.15, '░', Color::Rgb(60, 60, 80)),    // far
        (0.25, '▒', Color::Rgb(80, 80, 100)),   // mid-far
        (0.35, '▓', Color::Rgb(100, 100, 120)),  // mid
        (0.50, '█', Color::Rgb(130, 130, 150)),  // mid-near
        (0.70, '█', Color::Rgb(160, 160, 180)),  // near
    ];

    // Draw back wall (if no forward exit)
    if !has_north {
        let depth = 0.35; // back wall at medium distance
        let wall_top = (mid_y as f64 * (1.0 - depth)) as usize;
        let wall_bottom = h - wall_top;
        let wall_left = (mid_x as f64 * (1.0 - depth)) as usize;
        let wall_right = w - wall_left;

        for y in wall_top..wall_bottom {
            for x in wall_left..wall_right {
                if x < w && y < h {
                    buf[y][x] = ('▓', Color::Rgb(100, 90, 80));
                }
            }
        }
        // Door frame outline
        for x in wall_left..wall_right {
            if x < w {
                if wall_top < h { buf[wall_top][x] = ('▄', Color::Rgb(140, 130, 120)); }
                if wall_bottom < h { buf[wall_bottom.min(h-1)][x] = ('▀', Color::Rgb(140, 130, 120)); }
            }
        }
    } else {
        // Forward corridor opening — draw receding hallway
        for (i, &(depth, ch, color)) in layers.iter().enumerate().rev() {
            let wall_top = (mid_y as f64 * (1.0 - depth)) as usize;
            let wall_bottom = (h as f64 - mid_y as f64 * (1.0 - depth)) as usize;
            let wall_left = (mid_x as f64 * (1.0 - depth)) as usize;
            let wall_right = (w as f64 - mid_x as f64 * (1.0 - depth)) as usize;

            // Left wall slice
            let prev_left = if i + 1 < layers.len() {
                (mid_x as f64 * (1.0 - layers[i + 1].0)) as usize
            } else {
                0
            };
            for y in wall_top..wall_bottom.min(h) {
                for x in prev_left..wall_left.min(w) {
                    buf[y][x] = (ch, color);
                }
            }

            // Right wall slice
            let prev_right = if i + 1 < layers.len() {
                (w as f64 - mid_x as f64 * (1.0 - layers[i + 1].0)) as usize
            } else {
                w
            };
            for y in wall_top..wall_bottom.min(h) {
                for x in wall_right.min(w)..prev_right.min(w) {
                    buf[y][x] = (ch, color);
                }
            }
        }
    }

    // Draw left wall opening (west exit)
    if has_west {
        let opening_width = w / 5;
        let opening_top = h / 4;
        let opening_bottom = h * 3 / 4;
        for y in opening_top..opening_bottom {
            for x in 0..opening_width.min(w) {
                buf[y][x] = (' ', Color::Rgb(20, 20, 30));
            }
        }
        // Doorframe
        for y in opening_top..opening_bottom {
            if opening_width < w {
                buf[y][opening_width.min(w-1)] = ('│', Color::Rgb(150, 140, 130));
            }
        }
        if opening_width < w {
            for x in 0..opening_width.min(w) {
                buf[opening_top][x] = ('▄', Color::Rgb(150, 140, 130));
                buf[opening_bottom.min(h-1)][x] = ('▀', Color::Rgb(150, 140, 130));
            }
        }
        // "W" label
        if opening_width / 2 < w && (opening_top + 1) < h {
            buf[opening_top + 1][opening_width / 2] = ('W', Color::Yellow);
        }
    }

    // Draw right wall opening (east exit)
    if has_east {
        let opening_width = w / 5;
        let opening_start = w - opening_width;
        let opening_top = h / 4;
        let opening_bottom = h * 3 / 4;
        for y in opening_top..opening_bottom {
            for x in opening_start..w {
                buf[y][x] = (' ', Color::Rgb(20, 20, 30));
            }
        }
        // Doorframe
        for y in opening_top..opening_bottom {
            if opening_start > 0 {
                buf[y][opening_start] = ('│', Color::Rgb(150, 140, 130));
            }
        }
        for x in opening_start..w {
            buf[opening_top][x] = ('▄', Color::Rgb(150, 140, 130));
            buf[opening_bottom.min(h-1)][x] = ('▀', Color::Rgb(150, 140, 130));
        }
        // "E" label
        let label_x = opening_start + opening_width / 2;
        if label_x < w && (opening_top + 1) < h {
            buf[opening_top + 1][label_x] = ('E', Color::Yellow);
        }
    }

    // Draw forward exit indicator
    if has_north {
        let arrow_x = mid_x;
        let arrow_y = mid_y.saturating_sub(2);
        if arrow_y < h && arrow_x < w {
            buf[arrow_y][arrow_x] = ('N', Color::Yellow);
            if arrow_y + 1 < h { buf[arrow_y + 1][arrow_x] = ('↑', Color::Yellow); }
        }
    }

    // Draw behind indicator (south)
    if has_south {
        let arrow_x = mid_x;
        let arrow_y = h.saturating_sub(2);
        if arrow_y < h && arrow_x < w {
            buf[arrow_y][arrow_x] = ('S', Color::Rgb(100, 100, 60));
        }
    }

    // Draw monsters in the room
    let monster_chars = ['🐀', '👹', '🕷', '💀', '🎯'];
    // We can't use emoji easily in the char buffer — use ASCII instead
    // Place monster indicators in the center of the view
    if !state.room_name.is_empty() {
        // Check game log for recent monster sightings
        let monster_count = state.game_log.iter().rev().take(10)
            .filter(|l| l.contains("You see a"))
            .count();
        if monster_count > 0 {
            let mob_y = mid_y;
            let mob_x = mid_x;
            if mob_y < h && mob_x < w {
                buf[mob_y][mob_x] = ('M', Color::Red);
                if mob_x + 1 < w {
                    buf[mob_y][mob_x + 1] = ('!', Color::Red);
                }
            }
        }
    }

    // Render buffer to terminal
    for (row_idx, row) in buf.iter().enumerate() {
        if row_idx >= inner.height as usize {
            break;
        }
        let spans: Vec<Span> = row
            .iter()
            .take(inner.width as usize)
            .map(|&(ch, color)| Span::styled(ch.to_string(), Style::default().fg(color)))
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

fn ceiling_shade(dist: f64) -> char {
    if dist > 0.8 { ' ' }
    else if dist > 0.5 { '·' }
    else if dist > 0.3 { '∙' }
    else { '.' }
}

fn floor_shade(dist: f64) -> char {
    if dist > 0.8 { ' ' }
    else if dist > 0.5 { '·' }
    else if dist > 0.3 { '∙' }
    else { '.' }
}
