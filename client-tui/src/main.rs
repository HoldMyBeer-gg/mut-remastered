//! MUT Remastered — Native TUI Client
//!
//! Connects to the MUT server via TCP, handles login, character selection,
//! and presents a split-panel game interface using Ratatui.

mod app;
mod dungeon_view;
mod map;
mod net;
mod raycast;
mod ui;

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::prelude::*;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::mpsc;

use app::*;
use net::*;
use protocol::codec::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = std::env::var("MUT_SERVER").unwrap_or_else(|_| "127.0.0.1:4000".to_string());

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, crossterm::event::EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &addr).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, crossterm::event::DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e}");
    }
    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    addr: &str,
) -> anyhow::Result<()> {
    let mut screen = AppScreen::Connecting;

    // Draw connecting screen
    terminal.draw(|f| {
        let area = f.area();
        let msg = ratatui::widgets::Paragraph::new(format!("Connecting to {}...", addr))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Cyan));
        let centered = Layout::vertical([
            Constraint::Percentage(45),
            Constraint::Length(1),
            Constraint::Percentage(45),
        ])
        .split(area);
        f.render_widget(msg, centered[1]);
    })?;

    // Connect to server
    let (mut reader, writer) = net::connect(addr).await?;

    // Channel for server messages → UI
    let (server_tx, mut server_rx) = mpsc::channel::<ServerMessage>(64);
    // Channel for outgoing messages
    let (client_tx, mut client_rx) = mpsc::channel::<(u8, Vec<u8>)>(64);

    // Background reader task
    tokio::spawn(async move {
        loop {
            match read_frame(&mut reader).await {
                Ok(Some(payload)) => {
                    let msg = decode_server_message(&payload);
                    if server_tx.send(msg).await.is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(_) => break,
            }
        }
    });

    // Background writer task
    let writer_handle = tokio::spawn(async move {
        let mut writer: OwnedWriteHalf = writer;
        while let Some((_ns, full_frame)) = client_rx.recv().await {
            // full_frame is already [4-byte len][ns][postcard payload]
            if tokio::io::AsyncWriteExt::write_all(&mut writer, &full_frame)
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Move to login screen
    screen = AppScreen::Login(LoginState::new());

    loop {
        // Draw
        terminal.draw(|f| match &screen {
            AppScreen::Connecting => {}
            AppScreen::Login(state) => ui::render_login(f, state),
            AppScreen::CharacterSelect(state) => ui::render_char_select(f, state),
            AppScreen::InGame(state) => ui::render_game(f, state),
        })?;

        // Poll for events (input + server messages)
        // Use a short timeout to check server messages frequently
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    // Global quit: Ctrl-C
                    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c')
                    {
                        break;
                    }

                    match &mut screen {
                        AppScreen::Login(state) => {
                            handle_login_input(key.code, state, &client_tx).await;
                            if key.code == KeyCode::Esc {
                                break;
                            }
                        }
                        AppScreen::CharacterSelect(state) => {
                            let action =
                                handle_char_select_input(key.code, state, &client_tx).await;
                            if action == CharSelectAction::Quit {
                                break;
                            }
                        }
                        AppScreen::InGame(state) => {
                            handle_game_input(key.code, state, &client_tx).await;
                        }
                        _ => {}
                    }
                }
                Event::Mouse(mouse) => {
                    if let AppScreen::InGame(state) = &mut screen {
                        match mouse.kind {
                            crossterm::event::MouseEventKind::ScrollUp => {
                                state.log_scroll = state.log_scroll.saturating_add(3);
                            }
                            crossterm::event::MouseEventKind::ScrollDown => {
                                state.log_scroll = state.log_scroll.saturating_sub(3);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        // Process server messages
        while let Ok(msg) = server_rx.try_recv() {
            match handle_server_message(msg, &mut screen, &client_tx).await {
                MessageResult::Continue => {}
                MessageResult::Quit => break,
            }
        }
    }

    drop(client_tx);
    let _ = writer_handle.await;
    Ok(())
}

/// Send a protocol message through the client channel.
async fn send_msg<T: serde::Serialize>(tx: &mpsc::Sender<(u8, Vec<u8>)>, ns: u8, msg: &T) {
    // Use protocol's encode_message which returns [4-byte len][ns][postcard payload]
    // We need to send the raw bytes to the writer task which writes directly to TCP.
    let full_frame = protocol::codec::encode_message(ns, msg).unwrap();
    // The writer task expects (ns, postcard_payload), but let's simplify:
    // Send the full frame bytes directly. Redefine the channel to carry raw frames.
    // Actually, let's just send the full encoded frame and have the writer forward it.
    let _ = tx.send((0, full_frame)).await;
}

// ── Input Handlers ─────────────────────────────────────────────

async fn handle_login_input(
    key: KeyCode,
    state: &mut LoginState,
    tx: &mpsc::Sender<(u8, Vec<u8>)>,
) {
    match key {
        KeyCode::Tab => {
            state.focus = (state.focus + 1) % 2;
        }
        KeyCode::Char('r') | KeyCode::Char('R') if state.focus != 0 || state.username.is_empty() => {
            state.registering = !state.registering;
        }
        KeyCode::Char(c) => {
            if state.focus == 0 {
                state.username.push(c);
            } else {
                state.password.push(c);
            }
        }
        KeyCode::Backspace => {
            if state.focus == 0 {
                state.username.pop();
            } else {
                state.password.pop();
            }
        }
        KeyCode::Enter => {
            if state.username.is_empty() || state.password.is_empty() {
                state.error_message = Some("Username and password required".to_string());
                return;
            }
            state.error_message = None;
            if state.registering {
                send_msg(
                    tx,
                    NS_AUTH,
                    &protocol::auth::ClientMsg::Register {
                        username: state.username.clone(),
                        password: state.password.clone(),
                    },
                )
                .await;
            } else {
                send_msg(
                    tx,
                    NS_AUTH,
                    &protocol::auth::ClientMsg::Login {
                        username: state.username.clone(),
                        password: state.password.clone(),
                    },
                )
                .await;
            }
        }
        _ => {}
    }
}

#[derive(PartialEq)]
enum CharSelectAction {
    Continue,
    Quit,
}

async fn handle_char_select_input(
    key: KeyCode,
    state: &mut CharSelectState,
    tx: &mpsc::Sender<(u8, Vec<u8>)>,
) -> CharSelectAction {
    if state.creating {
        match key {
            KeyCode::Esc => {
                state.creating = false;
                state.error_message = None;
            }
            KeyCode::Tab => {
                state.create_focus = (state.create_focus + 1) % 4;
            }
            KeyCode::Char(c) if state.create_focus == 0 => {
                state.create_name.push(c);
            }
            KeyCode::Backspace if state.create_focus == 0 => {
                state.create_name.pop();
            }
            KeyCode::Left => match state.create_focus {
                1 => {
                    state.create_race =
                        (state.create_race + RACES.len() - 1) % RACES.len();
                }
                2 => {
                    state.create_class =
                        (state.create_class + CLASSES.len() - 1) % CLASSES.len();
                }
                3 => {
                    state.create_gender =
                        (state.create_gender + GENDERS.len() - 1) % GENDERS.len();
                }
                _ => {}
            },
            KeyCode::Right => match state.create_focus {
                1 => state.create_race = (state.create_race + 1) % RACES.len(),
                2 => state.create_class = (state.create_class + 1) % CLASSES.len(),
                3 => state.create_gender = (state.create_gender + 1) % GENDERS.len(),
                _ => {}
            },
            KeyCode::Enter => {
                if state.create_name.is_empty() {
                    state.error_message = Some("Name required".to_string());
                    return CharSelectAction::Continue;
                }
                // Standard point-buy: 15, 14, 13, 12, 10, 8 = 27 points
                let scores = [15, 14, 13, 12, 10, 8];
                let race = RACES[state.create_race];
                let racial_choices = match race {
                    "human" => vec![0, 1],    // +1 STR, +1 DEX
                    "half_elf" => vec![0],    // +1 STR
                    _ => vec![],
                };
                send_msg(
                    tx,
                    NS_CHAR,
                    &protocol::character::ClientMsg::CharacterCreate {
                        name: state.create_name.clone(),
                        race: race.to_string(),
                        class: CLASSES[state.create_class].to_string(),
                        gender: GENDERS[state.create_gender].to_string(),
                        ability_scores: scores,
                        racial_bonus_choices: racial_choices,
                    },
                )
                .await;
            }
            _ => {}
        }
        return CharSelectAction::Continue;
    }

    match key {
        KeyCode::Esc => return CharSelectAction::Quit,
        KeyCode::Char('c') | KeyCode::Char('C') => {
            state.creating = true;
            state.create_name.clear();
            state.create_focus = 0;
            state.error_message = None;
        }
        KeyCode::Up => {
            if state.selected_index > 0 {
                state.selected_index -= 1;
            }
        }
        KeyCode::Down => {
            if state.selected_index + 1 < state.characters.len() {
                state.selected_index += 1;
            }
        }
        KeyCode::Enter => {
            if let Some(c) = state.characters.get(state.selected_index) {
                send_msg(
                    tx,
                    NS_CHAR,
                    &protocol::character::ClientMsg::CharacterSelect {
                        character_id: c.id.clone(),
                    },
                )
                .await;
            }
        }
        _ => {}
    }
    CharSelectAction::Continue
}

async fn handle_game_input(
    key: KeyCode,
    state: &mut GameState,
    tx: &mpsc::Sender<(u8, Vec<u8>)>,
) {
    match key {
        KeyCode::Char(c) => {
            state.input.push(c);
        }
        KeyCode::Backspace => {
            state.input.pop();
        }
        KeyCode::Enter => {
            let cmd = state.input.trim().to_string();
            if cmd.is_empty() {
                return;
            }
            state.history.push(cmd.clone());
            state.history_index = None;
            state.input.clear();

            // Handle client-side commands first
            let lower_cmd = cmd.to_lowercase();
            let first_word = lower_cmd.split_whitespace().next().unwrap_or("");

            if first_word == "/help" || first_word == "help" || first_word == "?" || first_word == "/?" {
                state.log("".to_string());
                state.log("═══ MUT Remastered — Commands ═══".to_string());
                state.log("".to_string());
                state.log("── Movement (no / needed) ──".to_string());
                state.log("  n/s/e/w/u/d     Move in a direction".to_string());
                state.log("  /look (/l)      Look around the room".to_string());
                state.log("  /examine <thing> Examine something (/ex)".to_string());
                state.log("".to_string());
                state.log("── Combat ──".to_string());
                state.log("  /attack <target> Attack a monster (/a)".to_string());
                state.log("  /flee            Try to escape combat".to_string());
                state.log("  /blast           Mage: Arcane Blast (5 mana)".to_string());
                state.log("  /heal            Cleric: Heal self (5 mana)".to_string());
                state.log("  /strike          Warrior: Power Strike (5 stamina)".to_string());
                state.log("  /aim             Ranger: Aimed Shot (3 stamina)".to_string());
                state.log("  /sneak           Rogue: Sneak Attack (once/combat)".to_string());
                state.log("  /use <ability>   Use any ability by name".to_string());
                state.log("".to_string());
                state.log("── Inventory ──".to_string());
                state.log("  /inv             Show inventory (/i)".to_string());
                state.log("  /get <item>      Pick up item".to_string());
                state.log("  /drop <item>     Drop item".to_string());
                state.log("  /equip <item>    Equip item".to_string());
                state.log("  /unequip <slot>  Unequip slot".to_string());
                state.log("".to_string());
                state.log("── Social ──".to_string());
                state.log("  /say <text>      Room chat (IC)".to_string());
                state.log("  /gossip <text>   Global chat (OOC) (/g)".to_string());
                state.log("  /whisper <who> <text>  Private message (/w)".to_string());
                state.log("  /emote <text>    Emote (/me)".to_string());
                state.log("  /lookat <player> Inspect player".to_string());
                state.log("".to_string());
                state.log("── Info ──".to_string());
                state.log("  /stats           Character stats (or Tab key)".to_string());
                state.log("  /bio <text>      Set biography".to_string());
                state.log("  /desc <text>     Set description".to_string());
                state.log("  /quit            Log out".to_string());
                state.log("".to_string());
                state.log("── Interact (no / needed) ──".to_string());
                state.log("  pull lever       Interact with objects in rooms".to_string());
                state.log("  examine fountain Try interacting with things!".to_string());
                state.log("═════════════════════════════════════".to_string());
                return;
            }

            // Track move direction for map
            match first_word {
                "n" | "north" | "s" | "south" | "e" | "east" | "w" | "west" | "u" | "up" | "d" | "down" => {
                    // Normalize direction for tracking
                    let dir = match first_word {
                        "n" => "north", "s" => "south", "e" => "east", "w" => "west",
                        "u" => "up", "d" => "down",
                        other => other,
                    };
                    state.last_move_direction = Some(dir.to_string());
                }
                _ => {}
            }
            parse_and_send_command(&cmd, tx).await;
        }
        KeyCode::Up => {
            // Command history navigation
            if !state.history.is_empty() {
                let idx = match state.history_index {
                    Some(i) if i > 0 => i - 1,
                    Some(i) => i,
                    None => state.history.len() - 1,
                };
                state.history_index = Some(idx);
                state.input = state.history[idx].clone();
            }
        }
        KeyCode::Down => {
            if let Some(idx) = state.history_index {
                if idx + 1 < state.history.len() {
                    state.history_index = Some(idx + 1);
                    state.input = state.history[idx + 1].clone();
                } else {
                    state.history_index = None;
                    state.input.clear();
                }
            }
        }
        KeyCode::PageUp => {
            state.log_scroll = state.log_scroll.saturating_add(5);
        }
        KeyCode::PageDown => {
            state.log_scroll = state.log_scroll.saturating_sub(5);
        }
        KeyCode::F(1) => {
            state.log("".to_string());
            state.log("═══ Quick Help (F1) ═══".to_string());
            state.log("  n/s/e/w = move │ /look │ /attack <mob> │ /flee".to_string());
            state.log("  /blast /heal /strike /aim /sneak = class abilities".to_string());
            state.log("  /inv │ /get │ /drop │ /equip │ /stats │ /say │ /gossip".to_string());
            state.log("  Type /help for full command list".to_string());
            state.log("═══════════════════════".to_string());
        }
        KeyCode::Tab => {
            parse_and_send_command("/stats", tx).await;
        }
        _ => {}
    }
}

/// Parse a text command and send the appropriate protocol message.
/// Commands use / prefix (e.g., /look, /attack rat). Movement shortcuts (n/s/e/w/u/d) work without /.
async fn parse_and_send_command(cmd: &str, tx: &mpsc::Sender<(u8, Vec<u8>)>) {
    let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
    let verb = parts[0].to_lowercase();
    let arg = parts.get(1).map(|s| s.to_string()).unwrap_or_default();

    match verb.as_str() {
        // Movement — no / needed
        "n" | "north" | "s" | "south" | "e" | "east" | "w" | "west" | "u" | "up" | "d"
        | "down" => {
            send_msg(tx, NS_WORLD, &protocol::world::ClientMsg::Move { direction: verb.clone() }).await;
        }
        // All other commands use / prefix
        "/look" | "/l" => {
            send_msg(tx, NS_WORLD, &protocol::world::ClientMsg::Look).await;
        }
        "/examine" | "/exam" | "/ex" => {
            // Send as both Examine (for lore) and Interact (for triggers)
            send_msg(tx, NS_WORLD, &protocol::world::ClientMsg::Examine { target: arg.clone() }).await;
            send_msg(tx, NS_WORLD, &protocol::world::ClientMsg::Interact { command: format!("examine {}", arg) }).await;
        }
        "/attack" | "/kill" | "/hit" | "/a" => {
            send_msg(tx, NS_COMBAT, &protocol::combat::ClientMsg::Attack { target: arg }).await;
        }
        "/flee" | "/run" => {
            send_msg(tx, NS_COMBAT, &protocol::combat::ClientMsg::Flee).await;
        }
        "/use" | "/cast" => {
            send_msg(tx, NS_COMBAT, &protocol::combat::ClientMsg::UseAbility { ability_name: arg }).await;
        }
        "/blast" | "/arcane_blast" => {
            send_msg(tx, NS_COMBAT, &protocol::combat::ClientMsg::UseAbility { ability_name: "blast".to_string() }).await;
        }
        "/heal" => {
            send_msg(tx, NS_COMBAT, &protocol::combat::ClientMsg::UseAbility { ability_name: "heal".to_string() }).await;
        }
        "/strike" | "/power_strike" => {
            send_msg(tx, NS_COMBAT, &protocol::combat::ClientMsg::UseAbility { ability_name: "strike".to_string() }).await;
        }
        "/aim" | "/aimed_shot" => {
            send_msg(tx, NS_COMBAT, &protocol::combat::ClientMsg::UseAbility { ability_name: "aim".to_string() }).await;
        }
        "/sneak" | "/sneak_attack" => {
            send_msg(tx, NS_COMBAT, &protocol::combat::ClientMsg::UseAbility { ability_name: "sneak".to_string() }).await;
        }
        "/inv" | "/inventory" | "/i" => {
            send_msg(tx, NS_WORLD, &protocol::world::ClientMsg::Inventory).await;
        }
        "/get" | "/take" | "/pickup" => {
            send_msg(tx, NS_WORLD, &protocol::world::ClientMsg::GetItem { target: arg }).await;
        }
        "/drop" => {
            send_msg(tx, NS_WORLD, &protocol::world::ClientMsg::DropItem { target: arg }).await;
        }
        "/equip" | "/wear" | "/wield" => {
            send_msg(tx, NS_WORLD, &protocol::world::ClientMsg::Equip { item_name: arg }).await;
        }
        "/unequip" | "/remove" => {
            send_msg(tx, NS_WORLD, &protocol::world::ClientMsg::Unequip { slot: arg }).await;
        }
        "/stats" | "/score" | "/sc" => {
            send_msg(tx, NS_WORLD, &protocol::world::ClientMsg::Stats).await;
        }
        "/bio" => {
            send_msg(tx, NS_WORLD, &protocol::world::ClientMsg::Bio { text: arg }).await;
        }
        "/quit" | "/exit" => {
            send_msg(tx, NS_AUTH, &protocol::auth::ClientMsg::Logout).await;
        }
        "/say" => {
            send_msg(tx, NS_WORLD, &protocol::world::ClientMsg::Say { text: arg }).await;
        }
        "/emote" | "/em" | "/me" => {
            send_msg(tx, NS_WORLD, &protocol::world::ClientMsg::Emote { text: arg }).await;
        }
        "/whisper" | "/tell" | "/w" => {
            let wparts: Vec<&str> = arg.splitn(2, ' ').collect();
            if wparts.len() == 2 {
                send_msg(tx, NS_WORLD, &protocol::world::ClientMsg::Whisper {
                    target: wparts[0].to_string(),
                    text: wparts[1].to_string(),
                }).await;
            }
        }
        "/gossip" | "/ooc" | "/g" => {
            send_msg(tx, NS_WORLD, &protocol::world::ClientMsg::Gossip { text: arg }).await;
        }
        "/toggle" => {
            send_msg(tx, NS_WORLD, &protocol::world::ClientMsg::ToggleChannel { channel: arg }).await;
        }
        "/lookat" | "/inspect" => {
            send_msg(tx, NS_WORLD, &protocol::world::ClientMsg::LookAt { target: arg }).await;
        }
        "/describe" | "/desc" => {
            send_msg(tx, NS_WORLD, &protocol::world::ClientMsg::SetDescription { text: arg }).await;
        }
        "/descend" | "/enter" => {
            send_msg(tx, NS_WORLD, &protocol::world::ClientMsg::Interact { command: "enter dungeon".to_string() }).await;
        }
        _ => {
            // Try as interact command (freeform, no / needed)
            send_msg(
                tx,
                NS_WORLD,
                &protocol::world::ClientMsg::Interact {
                    command: cmd.to_string(),
                },
            )
            .await;
        }
    }
}

// ── Server Message Handler ──────────────────────────────────────

enum MessageResult {
    Continue,
    Quit,
}

async fn handle_server_message(
    msg: ServerMessage,
    screen: &mut AppScreen,
    tx: &mpsc::Sender<(u8, Vec<u8>)>,
) -> MessageResult {
    match msg {
        ServerMessage::Auth(auth_msg) => match auth_msg {
            protocol::auth::ServerMsg::RegisterOk { .. } => {
                if let AppScreen::Login(state) = screen {
                    state.error_message = Some("Registered! Now logging in...".to_string());
                    state.registering = false;
                    // Auto-login after register
                    send_msg(
                        tx,
                        NS_AUTH,
                        &protocol::auth::ClientMsg::Login {
                            username: state.username.clone(),
                            password: state.password.clone(),
                        },
                    )
                    .await;
                }
            }
            protocol::auth::ServerMsg::LoginOk { .. } => {
                // Transition to character select, request character list
                let mut cs = CharSelectState::new();
                send_msg(
                    tx,
                    NS_CHAR,
                    &protocol::character::ClientMsg::CharacterList,
                )
                .await;
                *screen = AppScreen::CharacterSelect(cs);
            }
            protocol::auth::ServerMsg::LogoutOk => {
                return MessageResult::Quit;
            }
            protocol::auth::ServerMsg::Error { message, .. } => {
                if let AppScreen::Login(state) = screen {
                    state.error_message = Some(message);
                }
            }
            _ => {}
        },

        ServerMessage::Character(char_msg) => match char_msg {
            protocol::character::ServerMsg::CharacterListResult { characters } => {
                if let AppScreen::CharacterSelect(state) = screen {
                    state.characters = characters;
                    state.selected_index = 0;
                }
            }
            protocol::character::ServerMsg::CharacterCreateOk { character_id, name } => {
                if let AppScreen::CharacterSelect(state) = screen {
                    state.creating = false;
                    state.error_message = None;
                    // Refresh list
                    send_msg(
                        tx,
                        NS_CHAR,
                        &protocol::character::ClientMsg::CharacterList,
                    )
                    .await;
                }
            }
            protocol::character::ServerMsg::CharacterCreateFail { reason } => {
                if let AppScreen::CharacterSelect(state) = screen {
                    state.error_message = Some(reason);
                }
            }
            protocol::character::ServerMsg::CharacterSelected {
                character_id,
                name,
            } => {
                *screen = AppScreen::InGame(GameState::new(character_id, name));
            }
            protocol::character::ServerMsg::CharacterSelectFail { reason } => {
                if let AppScreen::CharacterSelect(state) = screen {
                    state.error_message = Some(reason);
                }
            }
        },

        ServerMessage::World(world_msg) => {
            if let AppScreen::InGame(state) = screen {
                match world_msg {
                    protocol::world::ServerMsg::RoomDescription {
                        room_id,
                        name,
                        description,
                        exits,
                        hints,
                        players_here,
                        monsters_here,
                    } => {
                        let first_room = state.room_id.is_empty();
                        state.room_id = room_id;
                        state.room_name = name;
                        state.room_description = description;
                        state.room_exits = exits;
                        state.players_here = players_here;
                        state.record_room();
                        for hint in hints {
                            state.log(format!("💡 {}", hint));
                        }
                        if !monsters_here.is_empty() {
                            for m in &monsters_here {
                                state.log(format!("🐀 You see a {} here.", m));
                            }
                        }
                        if first_room {
                            state.log("".to_string());
                            state.log("⚔ Welcome to MUT Remastered v0.3! Type /help for commands.".to_string());
                            state.log("💡 Move: n/s/e/w │ Fight: /attack <mob> │ Ability: /blast /heal /strike".to_string());
                            state.log("".to_string());
                        }
                    }
                    protocol::world::ServerMsg::MoveOk { from_room, to_room } => {
                        // Record room connection for the map
                        if let Some(dir) = state.last_move_direction.take() {
                            state.room_connections.insert(
                                (from_room.clone(), dir.clone()),
                                to_room.clone(),
                            );
                            // Also record the reverse connection
                            let reverse_dir = match dir.as_str() {
                                "north" => "south", "south" => "north",
                                "east" => "west", "west" => "east",
                                "up" => "down", "down" => "up",
                                _ => "",
                            };
                            if !reverse_dir.is_empty() {
                                state.room_connections.insert(
                                    (to_room.clone(), reverse_dir.to_string()),
                                    from_room.clone(),
                                );
                            }
                        }
                    }
                    protocol::world::ServerMsg::MoveFail { reason } => {
                        state.log(format!("⛔ {}", reason));
                    }
                    protocol::world::ServerMsg::ExamineResult { text } => {
                        state.log(format!("📖 {}", text));
                    }
                    protocol::world::ServerMsg::InteractResult { text } => {
                        state.log(format!("⚙ {}", text));
                    }
                    protocol::world::ServerMsg::WorldEvent { message } => {
                        // Split multi-line messages (combat log sends \n-joined entries)
                        for line in message.lines() {
                            if !line.is_empty() {
                                state.log(line.to_string());
                            }
                        }
                    }
                    protocol::world::ServerMsg::InventoryList {
                        items,
                        equipped,
                        gold,
                    } => {
                        state.log("── Inventory ──".to_string());
                        if items.is_empty() && equipped.is_empty() {
                            state.log("  (empty)".to_string());
                        }
                        for item in &items {
                            state.log(format!("  📦 {}", item.name));
                        }
                        for eq in &equipped {
                            state.log(format!("  ⚔ [{}] {}", eq.slot, eq.name));
                        }
                        state.log(format!("  💰 {} gold", gold));
                    }
                    protocol::world::ServerMsg::GetItemOk { item_name } => {
                        state.log(format!("You pick up {}.", item_name));
                    }
                    protocol::world::ServerMsg::DropItemOk { item_name } => {
                        state.log(format!("You drop {}.", item_name));
                    }
                    protocol::world::ServerMsg::EquipOk { item_name, slot } => {
                        state.log(format!("You equip {} to [{}].", item_name, slot));
                    }
                    protocol::world::ServerMsg::UnequipOk { slot, item_name } => {
                        state.log(format!("You remove {} from [{}].", item_name, slot));
                    }
                    protocol::world::ServerMsg::StatsResult {
                        name,
                        race,
                        class,
                        level,
                        xp,
                        hp,
                        max_hp,
                        ac,
                        str_score,
                        dex_score,
                        con_score,
                        int_score,
                        wis_score,
                        cha_score,
                        bio,
                        ..
                    } => {
                        state.log("── Character Stats ──".to_string());
                        state.log(format!("  {} — {} {} Lv {}", name, race, class, level));
                        state.log(format!("  HP: {}/{} | AC: {} | XP: {}", hp, max_hp, ac, xp));
                        state.log(format!(
                            "  STR {} DEX {} CON {} INT {} WIS {} CHA {}",
                            str_score, dex_score, con_score, int_score, wis_score, cha_score
                        ));
                        if !bio.is_empty() {
                            state.log(format!("  Bio: {}", bio));
                        }
                    }
                    protocol::world::ServerMsg::BioOk => {
                        state.log("Biography updated.".to_string());
                    }
                    protocol::world::ServerMsg::WorldActionFail { reason } => {
                        state.log(format!("⛔ {}", reason));
                    }
                    protocol::world::ServerMsg::ChatMessage { channel, sender, text } => {
                        let prefix = if channel == "gossip" { "[OOC]" } else { "[IC]" };
                        state.log(format!("{} {} says: {}", prefix, sender, text));
                    }
                    protocol::world::ServerMsg::EmoteMessage { sender, text } => {
                        state.log(format!("[IC] {} {}", sender, text));
                    }
                    protocol::world::ServerMsg::WhisperMessage { from, text } => {
                        state.log(format!("[WHISPER] {} whispers: {}", from, text));
                    }
                    protocol::world::ServerMsg::WhisperSent { to, text } => {
                        state.log(format!("[WHISPER] You whisper to {}: {}", to, text));
                    }
                    protocol::world::ServerMsg::LookAtResult { name, race, class, level, description, bio, equipped } => {
                        state.log("── Player Info ──".to_string());
                        state.log(format!("  {} — {} {} Lv {}", name, race, class, level));
                        if !description.is_empty() {
                            state.log(format!("  {}", description));
                        }
                        if !bio.is_empty() {
                            state.log(format!("  Bio: {}", bio));
                        }
                        for eq in &equipped {
                            state.log(format!("  ⚔ [{}] {}", eq.slot, eq.name));
                        }
                    }
                    protocol::world::ServerMsg::DescriptionOk => {
                        state.log("Description updated.".to_string());
                    }
                    protocol::world::ServerMsg::ChannelToggled { channel, enabled } => {
                        let status = if enabled { "ON" } else { "OFF" };
                        state.log(format!("Channel '{}' is now {}.", channel, status));
                    }
                }
            }
        }

        ServerMessage::Combat(combat_msg) => {
            if let AppScreen::InGame(state) = screen {
                match combat_msg {
                    protocol::combat::ServerMsg::Vitals {
                        hp,
                        max_hp,
                        mana,
                        max_mana,
                        stamina,
                        max_stamina,
                        xp,
                        level,
                    } => {
                        state.hp = hp;
                        state.max_hp = max_hp;
                        state.mana = mana;
                        state.max_mana = max_mana;
                        state.stamina = stamina;
                        state.max_stamina = max_stamina;
                        state.xp = xp;
                        state.level = level;
                    }
                    protocol::combat::ServerMsg::CombatStart { combatants } => {
                        state.log(format!(
                            "⚔ Combat! Combatants: {}",
                            combatants.join(", ")
                        ));
                    }
                    protocol::combat::ServerMsg::CombatLog { entries } => {
                        for entry in entries {
                            state.log(entry);
                        }
                    }
                    protocol::combat::ServerMsg::CombatEnd { result } => {
                        state.log(format!("🏁 {}", result));
                    }
                    protocol::combat::ServerMsg::ActionFail { reason } => {
                        state.log(format!("⛔ {}", reason));
                    }
                }
            }
        }

        ServerMessage::Unknown => {}
    }
    MessageResult::Continue
}
