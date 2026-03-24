//! ASCII dungeon map renderer.
//!
//! Draws explored rooms as boxes on a 2D grid with corridors connecting them.
//! The player's current room is centered and marked with @.
//! Unexplored rooms adjacent to explored rooms show as ? (fog of war).

use std::collections::HashMap;

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::GameState;

/// Grid coordinates for room placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GridPos {
    x: i32,
    y: i32,
}

/// Room dimensions on the character grid.
const ROOM_W: i32 = 7;  // width of a room box (including borders)
const ROOM_H: i32 = 3;  // height of a room box
const CORR_LEN: i32 = 3; // corridor length between rooms
const CELL_W: i32 = ROOM_W + CORR_LEN; // total horizontal spacing
const CELL_H: i32 = ROOM_H + CORR_LEN; // total vertical spacing

/// Map cell contents for the character buffer.
#[derive(Clone, Copy, PartialEq)]
enum Cell {
    Empty,
    Wall,
    Floor,
    Door,
    Corridor,
    Player,
    Monster,
    Fog,         // unexplored adjacent room
    FogWall,
}

/// Render the dungeon map widget.
pub fn render_map(f: &mut Frame, state: &GameState, area: Rect) {
    let block = Block::bordered()
        .title(" Dungeon Map ")
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if state.room_id.is_empty() || inner.width < 5 || inner.height < 3 {
        return;
    }

    // Build room grid positions via BFS from current room
    let grid = build_grid(state);

    // Find the current room's grid position (always at origin)
    let player_grid = GridPos { x: 0, y: 0 };

    // Create character buffer
    let buf_w = inner.width as i32;
    let buf_h = inner.height as i32;
    let mut char_buf: Vec<Vec<(char, Color)>> = vec![vec![(' ', Color::DarkGray); buf_w as usize]; buf_h as usize];

    // Center offset: place player_grid at center of buffer
    let cx = buf_w / 2;
    let cy = buf_h / 2;

    // Draw rooms and corridors
    for (room_id, gpos) in &grid {
        let is_current = room_id == &state.room_id;
        let is_explored = state.explored_rooms.contains_key(room_id);

        // Screen position of this room's top-left corner
        let sx = cx + (gpos.x * CELL_W) - ROOM_W / 2;
        let sy = cy + (gpos.y * CELL_H) - ROOM_H / 2;

        if is_explored || is_current {
            draw_room(&mut char_buf, sx, sy, buf_w, buf_h, is_current);

            // Draw corridors to connected rooms
            if let Some(explored) = state.explored_rooms.get(room_id) {
                for exit in &explored.exits {
                    let (dx, dy) = dir_to_delta(exit);
                    draw_corridor(&mut char_buf, sx, sy, dx, dy, buf_w, buf_h);
                }
            }
        } else {
            // Fog of war — show as dim ?
            draw_fog_room(&mut char_buf, sx, sy, buf_w, buf_h);
        }
    }

    // Draw player marker
    let px = cx as usize;
    let py = cy as usize;
    if py < char_buf.len() && px < char_buf[0].len() {
        char_buf[py][px] = ('@', Color::Yellow);
    }

    // Render buffer to terminal
    for (row_idx, row) in char_buf.iter().enumerate() {
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

/// Build a grid of room positions using BFS from the current room.
fn build_grid(state: &GameState) -> HashMap<String, GridPos> {
    let mut grid: HashMap<String, GridPos> = HashMap::new();
    let mut queue: Vec<(String, GridPos)> = Vec::new();

    grid.insert(state.room_id.clone(), GridPos { x: 0, y: 0 });
    queue.push((state.room_id.clone(), GridPos { x: 0, y: 0 }));

    let mut visited = std::collections::HashSet::new();
    visited.insert(state.room_id.clone());

    while let Some((room_id, pos)) = queue.pop() {
        if let Some(explored) = state.explored_rooms.get(&room_id) {
            for exit in &explored.exits {
                let (dx, dy) = dir_to_delta(exit);
                if dx == 0 && dy == 0 {
                    continue; // up/down — skip for 2D map
                }
                // We don't know the target room_id from exits alone in the current data model.
                // So we look at all explored rooms and find which one is adjacent.
                // For now, create a placeholder position.
                let neighbor_pos = GridPos {
                    x: pos.x + dx,
                    y: pos.y + dy,
                };

                // Find the room at this grid position (if any explored room connects back)
                let neighbor_id = find_room_at_direction(state, &room_id, exit);
                if let Some(nid) = neighbor_id {
                    if !visited.contains(&nid) {
                        visited.insert(nid.clone());
                        grid.insert(nid.clone(), neighbor_pos);
                        queue.push((nid, neighbor_pos));
                    }
                } else {
                    // Unknown room (fog of war) — use a placeholder ID
                    let fog_id = format!("fog_{}_{}", neighbor_pos.x, neighbor_pos.y);
                    if !grid.contains_key(&fog_id) {
                        grid.insert(fog_id, neighbor_pos);
                    }
                }
            }
        }
    }

    grid
}

/// Try to find which room is connected via a given exit direction.
fn find_room_at_direction(state: &GameState, from_room: &str, direction: &str) -> Option<String> {
    state
        .room_connections
        .get(&(from_room.to_string(), direction.to_string()))
        .cloned()
}

/// Convert a direction string to grid delta.
fn dir_to_delta(dir: &str) -> (i32, i32) {
    match dir {
        "north" => (0, -1),
        "south" => (0, 1),
        "east" => (1, 0),
        "west" => (-1, 0),
        "up" | "down" => (0, 0), // not rendered on 2D map
        _ => (0, 0),
    }
}

/// Draw a room box at screen position (sx, sy).
fn draw_room(
    buf: &mut [Vec<(char, Color)>],
    sx: i32, sy: i32,
    buf_w: i32, buf_h: i32,
    is_current: bool,
) {
    let wall_color = if is_current { Color::Cyan } else { Color::White };
    let floor_color = if is_current { Color::Gray } else { Color::DarkGray };

    // Top wall: ┌─────┐
    set_cell(buf, sx, sy, '┌', wall_color, buf_w, buf_h);
    for x in 1..ROOM_W - 1 {
        set_cell(buf, sx + x, sy, '─', wall_color, buf_w, buf_h);
    }
    set_cell(buf, sx + ROOM_W - 1, sy, '┐', wall_color, buf_w, buf_h);

    // Middle: │·····│
    for y in 1..ROOM_H - 1 {
        set_cell(buf, sx, sy + y, '│', wall_color, buf_w, buf_h);
        for x in 1..ROOM_W - 1 {
            set_cell(buf, sx + x, sy + y, '·', floor_color, buf_w, buf_h);
        }
        set_cell(buf, sx + ROOM_W - 1, sy + y, '│', wall_color, buf_w, buf_h);
    }

    // Bottom wall: └─────┘
    set_cell(buf, sx, sy + ROOM_H - 1, '└', wall_color, buf_w, buf_h);
    for x in 1..ROOM_W - 1 {
        set_cell(buf, sx + x, sy + ROOM_H - 1, '─', wall_color, buf_w, buf_h);
    }
    set_cell(buf, sx + ROOM_W - 1, sy + ROOM_H - 1, '┘', wall_color, buf_w, buf_h);
}

/// Draw a fog-of-war room (unexplored).
fn draw_fog_room(
    buf: &mut [Vec<(char, Color)>],
    sx: i32, sy: i32,
    buf_w: i32, buf_h: i32,
) {
    let fog_color = Color::DarkGray;
    // Just draw a dim ? in the center
    let cx = sx + ROOM_W / 2;
    let cy = sy + ROOM_H / 2;
    set_cell(buf, cx, cy, '?', fog_color, buf_w, buf_h);
}

/// Draw a corridor from a room in a given direction.
fn draw_corridor(
    buf: &mut [Vec<(char, Color)>],
    room_sx: i32, room_sy: i32,
    dx: i32, dy: i32,
    buf_w: i32, buf_h: i32,
) {
    let color = Color::DarkGray;
    let mid_y = room_sy + ROOM_H / 2;
    let mid_x = room_sx + ROOM_W / 2;

    if dx > 0 {
        // East corridor
        let start_x = room_sx + ROOM_W;
        for i in 0..CORR_LEN {
            set_cell(buf, start_x + i, mid_y, '·', color, buf_w, buf_h);
        }
        // Door on room wall
        set_cell(buf, room_sx + ROOM_W - 1, mid_y, '╡', Color::Yellow, buf_w, buf_h);
    } else if dx < 0 {
        // West corridor
        let start_x = room_sx - CORR_LEN;
        for i in 0..CORR_LEN {
            set_cell(buf, start_x + i, mid_y, '·', color, buf_w, buf_h);
        }
        set_cell(buf, room_sx, mid_y, '╞', Color::Yellow, buf_w, buf_h);
    } else if dy < 0 {
        // North corridor
        let start_y = room_sy - CORR_LEN;
        for i in 0..CORR_LEN {
            set_cell(buf, mid_x, start_y + i, '·', color, buf_w, buf_h);
        }
        set_cell(buf, mid_x, room_sy, '╤', Color::Yellow, buf_w, buf_h);
    } else if dy > 0 {
        // South corridor
        let start_y = room_sy + ROOM_H;
        for i in 0..CORR_LEN {
            set_cell(buf, mid_x, start_y + i, '·', color, buf_w, buf_h);
        }
        set_cell(buf, mid_x, room_sy + ROOM_H - 1, '╧', Color::Yellow, buf_w, buf_h);
    }
}

/// Set a character in the buffer (bounds-checked).
fn set_cell(
    buf: &mut [Vec<(char, Color)>],
    x: i32, y: i32,
    ch: char, color: Color,
    buf_w: i32, buf_h: i32,
) {
    if x >= 0 && y >= 0 && x < buf_w && y < buf_h {
        buf[y as usize][x as usize] = (ch, color);
    }
}
