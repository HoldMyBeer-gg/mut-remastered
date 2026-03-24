//! TUI rendering functions using Ratatui.

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::*;

/// Render the login screen.
pub fn render_login(f: &mut Frame, state: &LoginState) {
    let area = f.area();

    // Center a box
    let popup = centered_rect(50, 40, area);
    f.render_widget(Clear, popup);

    let block = Block::bordered()
        .title(" MUT Remastered — Login ")
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::vertical([
        Constraint::Length(1), // spacer
        Constraint::Length(1), // "Username:"
        Constraint::Length(3), // input
        Constraint::Length(1), // "Password:"
        Constraint::Length(3), // input
        Constraint::Length(1), // spacer
        Constraint::Length(1), // instructions
        Constraint::Length(1), // error
        Constraint::Min(0),
    ])
    .split(inner);

    let username_style = if state.focus == 0 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };
    let password_style = if state.focus == 1 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };

    f.render_widget(Paragraph::new("Username:").style(username_style), chunks[1]);
    let username_block = Block::bordered().border_type(BorderType::Rounded).style(username_style);
    let username_text = Paragraph::new(state.username.as_str()).block(username_block);
    f.render_widget(username_text, chunks[2]);

    f.render_widget(Paragraph::new("Password:").style(password_style), chunks[3]);
    let password_block = Block::bordered().border_type(BorderType::Rounded).style(password_style);
    let masked: String = "*".repeat(state.password.len());
    let password_text = Paragraph::new(masked).block(password_block);
    f.render_widget(password_text, chunks[4]);

    let mode = if state.registering { "Register" } else { "Login" };
    let instructions = Paragraph::new(format!(
        "Tab: switch field | Enter: {} | R: toggle register | Esc: quit", mode
    ))
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Center);
    f.render_widget(instructions, chunks[6]);

    if let Some(ref err) = state.error_message {
        let err_text = Paragraph::new(err.as_str())
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center);
        f.render_widget(err_text, chunks[7]);
    }
}

/// Render character selection screen.
pub fn render_char_select(f: &mut Frame, state: &CharSelectState) {
    let area = f.area();

    if state.creating {
        render_char_create(f, state);
        return;
    }

    let popup = centered_rect(60, 60, area);
    f.render_widget(Clear, popup);

    let block = Block::bordered()
        .title(" Select Character ")
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    if state.characters.is_empty() {
        let msg = Paragraph::new("No characters yet. Press 'C' to create one.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(msg, inner);
        return;
    }

    let items: Vec<ListItem> = state
        .characters
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let style = if i == state.selected_index {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if i == state.selected_index { "▶ " } else { "  " };
            ListItem::new(format!(
                "{}{} — {} {} (Lv {})",
                prefix, c.name, c.race, c.class, c.level
            ))
            .style(style)
        })
        .collect();

    let chunks = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(2),
    ])
    .split(inner);

    let list = List::new(items);
    f.render_widget(list, chunks[0]);

    let instructions = Paragraph::new("↑/↓: select | Enter: play | C: create new | Esc: quit")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(instructions, chunks[1]);

    if let Some(ref err) = state.error_message {
        // Show at bottom
        let err_text = Paragraph::new(err.as_str())
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center);
        f.render_widget(err_text, chunks[1]);
    }
}

/// Render character creation form.
fn render_char_create(f: &mut Frame, state: &CharSelectState) {
    let area = f.area();
    let popup = centered_rect(60, 60, area);
    f.render_widget(Clear, popup);

    let block = Block::bordered()
        .title(" Create Character ")
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::Green));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(3), // name
        Constraint::Length(1), // race label
        Constraint::Length(1), // race
        Constraint::Length(1), // class label
        Constraint::Length(1), // class
        Constraint::Length(1), // gender label
        Constraint::Length(1), // gender
        Constraint::Length(1), // spacer
        Constraint::Length(1), // instructions
        Constraint::Length(1), // error
        Constraint::Min(0),
    ])
    .split(inner);

    let highlight = |focused: bool| {
        if focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        }
    };

    f.render_widget(Paragraph::new("Name:").style(highlight(state.create_focus == 0)), chunks[1]);
    let name_block = Block::bordered().style(highlight(state.create_focus == 0));
    f.render_widget(
        Paragraph::new(state.create_name.as_str()).block(name_block),
        chunks[1],
    );

    f.render_widget(
        Paragraph::new(format!("Race: ◄ {} ►", RACE_NAMES[state.create_race]))
            .style(highlight(state.create_focus == 1)),
        chunks[3],
    );

    f.render_widget(
        Paragraph::new(format!("Class: ◄ {} ►", CLASS_NAMES[state.create_class]))
            .style(highlight(state.create_focus == 2)),
        chunks[5],
    );

    f.render_widget(
        Paragraph::new(format!("Gender: ◄ {} ►", GENDER_NAMES[state.create_gender]))
            .style(highlight(state.create_focus == 3)),
        chunks[7],
    );

    let instructions = Paragraph::new("Tab: next field | ←/→: change | Enter: create | Esc: back")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(instructions, chunks[9]);

    if let Some(ref err) = state.error_message {
        let err_text = Paragraph::new(err.as_str())
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center);
        f.render_widget(err_text, chunks[10]);
    }
}

/// Render the main game screen.
pub fn render_game(f: &mut Frame, state: &GameState) {
    let area = f.area();

    // Main layout: menu bar, top section (room + map), game log, vitals bar, input
    let main_chunks = Layout::vertical([
        Constraint::Length(1),       // menu bar
        Constraint::Percentage(30),  // room + map
        Constraint::Min(5),          // game log
        Constraint::Length(4),       // vitals bar
        Constraint::Length(3),       // input
    ])
    .split(area);

    // Menu bar
    render_menu_bar(f, main_chunks[0]);

    // Top section: room description (left 50%) + dungeon map (right 50%)
    let top_chunks = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(main_chunks[1]);

    // Room description pane
    render_room_pane(f, state, top_chunks[0]);

    // Dungeon map pane
    crate::map::render_map(f, state, top_chunks[1]);

    // Game log
    render_game_log(f, state, main_chunks[2]);

    // Vitals bar
    render_vitals(f, state, main_chunks[3]);

    // Input line
    render_input(f, state, main_chunks[4]);
}

fn render_menu_bar(f: &mut Frame, area: Rect) {
    let menu = Line::from(vec![
        Span::styled(" ⚔ MUT ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::styled("│", Style::default().fg(Color::DarkGray)),
        Span::styled(" F1", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(":Help ", Style::default().fg(Color::Gray)),
        Span::styled("│", Style::default().fg(Color::DarkGray)),
        Span::styled(" /help", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(":Full ", Style::default().fg(Color::Gray)),
        Span::styled("│", Style::default().fg(Color::DarkGray)),
        Span::styled(" Tab", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(":Stats ", Style::default().fg(Color::Gray)),
        Span::styled("│", Style::default().fg(Color::DarkGray)),
        Span::styled(" PgUp/Dn", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(":Scroll ", Style::default().fg(Color::Gray)),
        Span::styled("│", Style::default().fg(Color::DarkGray)),
        Span::styled(" ↑↓", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(":History ", Style::default().fg(Color::Gray)),
        Span::styled("│", Style::default().fg(Color::DarkGray)),
        Span::styled(" 🖱Scroll", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(":Log ", Style::default().fg(Color::Gray)),
        Span::styled("│", Style::default().fg(Color::DarkGray)),
        Span::styled(" Ctrl-C", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(":Quit ", Style::default().fg(Color::Gray)),
    ]);

    let bar = Paragraph::new(menu)
        .style(Style::default().bg(Color::Rgb(20, 20, 40)));
    f.render_widget(bar, area);
}

fn render_room_pane(f: &mut Frame, state: &GameState, area: Rect) {
    let block = Block::bordered()
        .title(format!(" {} ", state.room_name))
        .title_alignment(Alignment::Left)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::Cyan));

    let mut text = vec![
        Line::from(state.room_description.clone()).style(Style::default().fg(Color::White)),
        Line::from(""),
    ];

    if !state.room_exits.is_empty() {
        text.push(
            Line::from(format!("Exits: {}", state.room_exits.join(", ")))
                .style(Style::default().fg(Color::Green)),
        );
    }

    if !state.players_here.is_empty() {
        text.push(
            Line::from(format!("Also here: {}", state.players_here.join(", ")))
                .style(Style::default().fg(Color::Yellow)),
        );
    }

    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    f.render_widget(paragraph, area);
}

fn render_minimap(f: &mut Frame, state: &GameState, area: Rect) {
    let block = Block::bordered()
        .title(" Compass ")
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::Magenta));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Simple compass showing available exits
    let exits = &state.room_exits;
    let n = if exits.iter().any(|e| e == "north") { "N" } else { "·" };
    let s = if exits.iter().any(|e| e == "south") { "S" } else { "·" };
    let e = if exits.iter().any(|e| e == "east") { "E" } else { "·" };
    let w = if exits.iter().any(|e| e == "west") { "W" } else { "·" };
    let u = if exits.iter().any(|e| e == "up") { "U" } else { " " };
    let d = if exits.iter().any(|e| e == "down") { "D" } else { " " };

    let compass_lines = vec![
        Line::from(format!("      {}  {}", n, u)).style(Style::default().fg(Color::White)),
        Line::from(format!("    {} ◆ {}", w, e)).style(Style::default().fg(Color::White)),
        Line::from(format!("      {}  {}", s, d)).style(Style::default().fg(Color::White)),
        Line::from(""),
        Line::from(format!("Rooms: {}", state.explored_rooms.len()))
            .style(Style::default().fg(Color::DarkGray)),
    ];

    let compass = Paragraph::new(compass_lines).alignment(Alignment::Center);
    f.render_widget(compass, inner);
}

fn render_game_log(f: &mut Frame, state: &GameState, area: Rect) {
    let block = Block::bordered()
        .title(" Game Log ")
        .title_alignment(Alignment::Left)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::White));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines: Vec<Line> = state
        .game_log
        .iter()
        .map(|entry| {
            let style = if entry.starts_with("CRITICAL") || entry.contains("HIT!") {
                Style::default().fg(Color::Red)
            } else if entry.contains("MISS") {
                Style::default().fg(Color::DarkGray)
            } else if entry.contains("slain") || entry.contains("died") {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            } else if entry.contains("Victory") {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else if entry.starts_with("💡") || entry.starts_with("⚔") {
                Style::default().fg(Color::Cyan)
            } else if entry.starts_with("🐀") {
                Style::default().fg(Color::Yellow)
            } else if entry.starts_with("═") || entry.starts_with("──") {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            Line::styled(entry.as_str(), style)
        })
        .collect();

    // Auto-scroll: show last N lines that fit in the area
    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((state.log_scroll, 0));

    // Calculate scroll to show bottom
    // We want to show the most recent entries
    let content_height = state.game_log.len() as u16;
    let view_height = inner.height;
    let scroll = if content_height > view_height {
        content_height - view_height + state.log_scroll
    } else {
        0
    };

    let paragraph = Paragraph::new(
        state.game_log.iter().map(|entry| {
            let style = if entry.starts_with("CRITICAL") || entry.contains("HIT!") {
                Style::default().fg(Color::Red)
            } else if entry.contains("MISS") {
                Style::default().fg(Color::DarkGray)
            } else if entry.contains("slain") || entry.contains("died") {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            } else if entry.contains("Victory") {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else if entry.starts_with("💡") || entry.starts_with("⚔") {
                Style::default().fg(Color::Cyan)
            } else if entry.starts_with("🐀") {
                Style::default().fg(Color::Yellow)
            } else if entry.starts_with("═") || entry.starts_with("──") {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            Line::styled(entry.as_str(), style)
        }).collect::<Vec<Line>>()
    )
    .wrap(Wrap { trim: false })
    .scroll((scroll, 0));

    f.render_widget(paragraph, inner);
}

fn render_vitals(f: &mut Frame, state: &GameState, area: Rect) {
    let block = Block::new()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(format!(" {} — Lv {} — XP: {} ", state.character_name, state.level, state.xp))
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let bar_chunks = Layout::horizontal([
        Constraint::Percentage(33),
        Constraint::Percentage(33),
        Constraint::Percentage(34),
    ])
    .split(inner);

    // HP bar
    let hp_ratio = if state.max_hp > 0 { state.hp as f64 / state.max_hp as f64 } else { 0.0 };
    let hp_color = if hp_ratio > 0.6 { Color::Green } else if hp_ratio > 0.3 { Color::Yellow } else { Color::Red };
    let hp_gauge = Gauge::default()
        .gauge_style(Style::default().fg(hp_color).bg(Color::Rgb(30, 30, 30)))
        .label(Span::styled(
            format!("HP {}/{}", state.hp, state.max_hp),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ))
        .ratio(hp_ratio.clamp(0.0, 1.0));
    f.render_widget(hp_gauge, bar_chunks[0]);

    // Mana bar
    let mp_ratio = if state.max_mana > 0 { state.mana as f64 / state.max_mana as f64 } else { 0.0 };
    let mp_gauge = Gauge::default()
        .gauge_style(Style::default().fg(Color::Blue).bg(Color::Rgb(30, 30, 30)))
        .label(Span::styled(
            format!("MP {}/{}", state.mana, state.max_mana),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ))
        .ratio(mp_ratio.clamp(0.0, 1.0));
    f.render_widget(mp_gauge, bar_chunks[1]);

    // Stamina bar
    let sp_ratio = if state.max_stamina > 0 { state.stamina as f64 / state.max_stamina as f64 } else { 0.0 };
    let sp_gauge = Gauge::default()
        .gauge_style(Style::default().fg(Color::Yellow).bg(Color::Rgb(30, 30, 30)))
        .label(Span::styled(
            format!("SP {}/{}", state.stamina, state.max_stamina),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ))
        .ratio(sp_ratio.clamp(0.0, 1.0));
    f.render_widget(sp_gauge, bar_chunks[2]);
}

fn render_input(f: &mut Frame, state: &GameState, area: Rect) {
    let block = Block::bordered()
        .title(" Command ")
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::Cyan));

    let input = Paragraph::new(state.input.as_str())
        .block(block)
        .style(Style::default().fg(Color::White));
    f.render_widget(input, area);

    // Place cursor
    f.set_cursor_position((area.x + 1 + state.input.len() as u16, area.y + 1));
}

/// Helper to create a centered rectangle.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
