mod helpers;

use protocol::world::ServerMsg as WorldServerMsg;

// ── WRLD-01: Look returns room description ──────────────────────────────────

/// WRLD-01: After login + character select, a Look command returns a RoomDescription
/// with non-empty name, description, and exits list.
#[tokio::test]
async fn test_look_returns_room_description() {
    let server = helpers::TestServer::start_with_world().await;
    let mut client = server.connect().await;

    let username = format!("user_{}", uuid::Uuid::new_v4().simple());
    client.full_setup(&username, "password", "TestLook").await;

    client.send_look().await;
    let msg = client.recv_world().await;

    match msg {
        WorldServerMsg::RoomDescription {
            name,
            description,
            exits,
            ..
        } => {
            assert!(!name.is_empty(), "room name should not be empty");
            assert!(!description.is_empty(), "room description should not be empty");
            assert!(!exits.is_empty(), "starting room should have at least one exit");
        }
        other => panic!("expected RoomDescription, got {:?}", other),
    }
}

// ── WRLD-02: Movement ────────────────────────────────────────────────────────

/// WRLD-02: Moving north from the default spawn room succeeds.
#[tokio::test]
async fn test_move_cardinal_direction() {
    let server = helpers::TestServer::start_with_world().await;
    let mut client = server.connect().await;

    let username = format!("user_{}", uuid::Uuid::new_v4().simple());
    client.full_setup(&username, "password", "TestMove").await;

    // Find starting room via look
    client.send_look().await;
    let start_desc = client.recv_world().await;
    let start_room_id = match &start_desc {
        WorldServerMsg::RoomDescription { room_id, .. } => room_id.clone(),
        other => panic!("expected RoomDescription, got {:?}", other),
    };

    // Move north
    client.send_move("north").await;

    let move_resp = client.recv_world().await;
    match &move_resp {
        WorldServerMsg::MoveOk { from_room, to_room } => {
            assert_eq!(from_room, &start_room_id);
            assert_ne!(to_room, &start_room_id);
        }
        other => panic!("expected MoveOk, got {:?}", other),
    }

    // Auto-look response after move
    let room_desc = client.recv_world().await;
    match room_desc {
        WorldServerMsg::RoomDescription { room_id, name, description, .. } => {
            assert_ne!(room_id, start_room_id);
            assert!(!name.is_empty());
            assert!(!description.is_empty());
        }
        other => panic!("expected RoomDescription after move, got {:?}", other),
    }
}

/// WRLD-02: Single-letter direction alias 'n' behaves the same as 'north'.
#[tokio::test]
async fn test_move_alias() {
    let server = helpers::TestServer::start_with_world().await;
    let mut client = server.connect().await;

    let username = format!("user_{}", uuid::Uuid::new_v4().simple());
    client.full_setup(&username, "password", "TestAlias").await;

    client.send_move("n").await;

    let move_resp = client.recv_world().await;
    match move_resp {
        WorldServerMsg::MoveOk { .. } => {}
        WorldServerMsg::MoveFail { reason } => {
            panic!("'n' alias failed with: {}", reason)
        }
        other => panic!("expected MoveOk, got {:?}", other),
    }
}

/// WRLD-02: Moving in a direction with no exit returns MoveFail.
#[tokio::test]
async fn test_move_no_exit() {
    let server = helpers::TestServer::start_with_world().await;
    let mut client = server.connect().await;

    let username = format!("user_{}", uuid::Uuid::new_v4().simple());
    client.full_setup(&username, "password", "TestNoExit").await;

    client.send_move("up").await;

    let resp = client.recv_world().await;
    match resp {
        WorldServerMsg::MoveFail { .. } => {}
        other => panic!("expected MoveFail, got {:?}", other),
    }
}

// ── WRLD-03: Position persistence across restart ─────────────────────────────

/// WRLD-03: A character's position is preserved across server restarts.
#[tokio::test]
async fn test_position_survives_restart() {
    let db_path = tempfile::NamedTempFile::new()
        .expect("failed to create tempfile")
        .into_temp_path();
    let db_url = format!("sqlite:{}", db_path.to_str().unwrap());

    let username = format!("user_{}", uuid::Uuid::new_v4().simple());
    let password = "persist_test";
    let char_name = "PersistHero";

    // Server 1: full setup, move north, disconnect
    let (new_room_id, character_id) = {
        let server = helpers::TestServer::start_with_db(&db_url).await;
        let mut client = server.connect().await;
        let (_token, char_id) = client.full_setup(&username, password, char_name).await;

        // Move north from spawn room
        client.send_move("north").await;
        let move_resp = client.recv_world().await;
        let new_room = match move_resp {
            WorldServerMsg::MoveOk { to_room, .. } => to_room,
            other => panic!("expected MoveOk, got {:?}", other),
        };
        let _desc = client.recv_world().await; // consume auto-look
        (new_room, char_id)
    };

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Server 2: same DB, login, select character, look → should be in new_room_id
    let server2 = helpers::TestServer::start_with_db(&db_url).await;
    let mut client2 = server2.connect().await;

    // Login (account already exists)
    client2
        .send(&protocol::auth::ClientMsg::Login {
            username: username.clone(),
            password: password.to_string(),
        })
        .await;
    let login_resp = client2.recv().await;
    assert!(
        matches!(login_resp, protocol::auth::ServerMsg::LoginOk { .. }),
        "expected LoginOk on second server, got {:?}",
        login_resp
    );

    // Select character (already exists in DB)
    client2
        .send_char(&protocol::character::ClientMsg::CharacterSelect {
            character_id: character_id.clone(),
        })
        .await;
    let _selected = client2.recv_char().await; // CharacterSelected
    let room_desc = client2.recv_world().await; // Initial RoomDescription

    match room_desc {
        WorldServerMsg::RoomDescription { room_id, .. } => {
            assert_eq!(
                room_id, new_room_id,
                "character should be in the room they moved to before restart"
            );
        }
        other => panic!("expected RoomDescription, got {:?}", other),
    }
}

// ── WRLD-04: Tutorial hints ──────────────────────────────────────────────────

/// WRLD-04: A new player sees tutorial hints.
#[tokio::test]
async fn test_hints_shown_to_new_player() {
    let server = helpers::TestServer::start_with_world().await;
    let mut client = server.connect().await;

    let username = format!("user_{}", uuid::Uuid::new_v4().simple());
    client.full_setup(&username, "password", "NewbieHints").await;

    client.send_look().await;
    let resp = client.recv_world().await;
    match resp {
        WorldServerMsg::RoomDescription { hints, .. } => {
            assert!(
                !hints.is_empty(),
                "new player should see tutorial hints in starting room"
            );
        }
        other => panic!("expected RoomDescription, got {:?}", other),
    }
}

/// WRLD-04: After completing the tutorial, hints are suppressed.
#[tokio::test]
async fn test_hints_suppressed_after_tutorial() {
    let server2 = helpers::TestServer::start_with_spawn("newbie:entrance").await;
    let mut client2 = server2.connect().await;

    let username = format!("user_{}", uuid::Uuid::new_v4().simple());
    client2.full_setup(&username, "password", "TutorialDone").await;

    // Navigate: entrance -> north -> courtyard -> west -> garden
    client2.send_move("north").await;
    let _ = client2.recv_world().await; // MoveOk
    let _ = client2.recv_world().await; // RoomDescription (courtyard)

    client2.send_move("west").await;
    let _ = client2.recv_world().await; // MoveOk
    let _ = client2.recv_world().await; // RoomDescription (garden)

    // Fire the tutorial completion trigger
    client2.send_interact("pass through archway").await;
    let interact_resp = client2.recv_world().await;
    match &interact_resp {
        WorldServerMsg::InteractResult { text } => {
            assert!(!text.is_empty(), "tutorial completion trigger should return a message");
        }
        other => panic!("expected InteractResult, got {:?}", other),
    }

    // Now look — hints should be suppressed
    client2.send_look().await;
    let look_resp = client2.recv_world().await;
    match look_resp {
        WorldServerMsg::RoomDescription { hints, .. } => {
            assert!(hints.is_empty(), "hints should be suppressed after tutorial completion");
        }
        other => panic!("expected RoomDescription, got {:?}", other),
    }
}

// ── WRLD-05: Examine lore ────────────────────────────────────────────────────

/// WRLD-05: Examining a target with lore returns the lore text.
#[tokio::test]
async fn test_examine_returns_lore() {
    let server = helpers::TestServer::start_with_world().await;
    let mut client = server.connect().await;

    let username = format!("user_{}", uuid::Uuid::new_v4().simple());
    client.full_setup(&username, "password", "LoreExplorer").await;

    client.send_examine("room").await;
    let resp = client.recv_world().await;
    match resp {
        WorldServerMsg::ExamineResult { text } => {
            assert!(
                !text.is_empty() && text != "You find nothing of note.",
                "examine 'room' in a room with lore should return lore, got: {:?}",
                text
            );
        }
        other => panic!("expected ExamineResult, got {:?}", other),
    }
}

/// WRLD-05: Examining a nonexistent target returns "nothing of note".
#[tokio::test]
async fn test_examine_unknown_target() {
    let server = helpers::TestServer::start_with_world().await;
    let mut client = server.connect().await;

    let username = format!("user_{}", uuid::Uuid::new_v4().simple());
    client.full_setup(&username, "password", "UnknownExamine").await;

    client.send_examine("nonexistent_purple_dragon_statue").await;
    let resp = client.recv_world().await;
    match resp {
        WorldServerMsg::ExamineResult { text } => {
            assert!(text.to_lowercase().contains("nothing of note"));
        }
        other => panic!("expected ExamineResult, got {:?}", other),
    }
}

// ── WRLD-06: Triggers ────────────────────────────────────────────────────────

/// WRLD-06: A trigger fires and its state change persists.
#[tokio::test]
async fn test_trigger_fires_and_persists() {
    let server = helpers::TestServer::start_with_spawn("newbie:practice_hall").await;
    let mut client = server.connect().await;

    let username = format!("user_{}", uuid::Uuid::new_v4().simple());
    client.full_setup(&username, "password", "TriggerTest").await;

    // First pull
    client.send_interact("pull lever").await;
    let first_resp = client.recv_world().await;
    match &first_resp {
        WorldServerMsg::InteractResult { text } => {
            assert!(
                text.contains("lever") || text.contains("door") || text.contains("clunk"),
                "first pull should return trigger message, got: {:?}",
                text
            );
        }
        other => panic!("expected InteractResult on first pull, got {:?}", other),
    }

    // Drain the WorldEvent broadcast
    let broadcast_event = client.recv_world().await;
    assert!(
        matches!(broadcast_event, WorldServerMsg::WorldEvent { .. }),
        "expected WorldEvent broadcast, got {:?}",
        broadcast_event
    );

    // Second pull: lever_state is now "true", condition fails
    client.send_interact("pull lever").await;
    let second_resp = client.recv_world().await;
    match &second_resp {
        WorldServerMsg::InteractResult { text } => {
            assert!(
                text.contains("already") || text.contains("lever"),
                "second pull should indicate lever already pulled, got: {:?}",
                text
            );
        }
        other => panic!("expected InteractResult on second pull, got {:?}", other),
    }
}

/// WRLD-06: A trigger broadcast is received by other players in the room.
#[tokio::test]
async fn test_trigger_broadcasts_to_room() {
    let server = helpers::TestServer::start_with_spawn("newbie:courtyard").await;

    let mut player_a = server.connect().await;
    let mut player_b = server.connect().await;

    let username_a = format!("player_a_{}", uuid::Uuid::new_v4().simple());
    let username_b = format!("player_b_{}", uuid::Uuid::new_v4().simple());
    let char_name_a = "BroadcastAlpha";
    let char_name_b = "BroadcastBeta";

    player_a.full_setup(&username_a, "password", &char_name_a).await;
    player_b.full_setup(&username_b, "password", &char_name_b).await;

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Player A fires the fountain trigger
    player_a.send_interact("examine fountain").await;
    let a_resp = player_a.recv_world().await;
    match &a_resp {
        WorldServerMsg::InteractResult { text } => {
            assert!(!text.is_empty(), "Player A should receive trigger message");
        }
        other => panic!("expected InteractResult for Player A, got {:?}", other),
    }

    // Player B should receive a WorldEvent broadcast
    let b_event = player_b.recv_world().await;
    match b_event {
        WorldServerMsg::WorldEvent { message } => {
            assert!(
                message.contains(char_name_a) || message.contains("fountain") || message.contains("glow"),
                "broadcast should reference the character name or the event, got: {:?}",
                message
            );
        }
        other => panic!("expected WorldEvent for Player B, got {:?}", other),
    }
}
