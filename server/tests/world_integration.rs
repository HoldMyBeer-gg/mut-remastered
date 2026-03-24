mod helpers;

use protocol::world::ServerMsg as WorldServerMsg;

// ── WRLD-01: Look returns room description ──────────────────────────────────

/// WRLD-01: After login, a Look command returns a RoomDescription with non-empty
/// name, description, and exits list.
#[tokio::test]
async fn test_look_returns_room_description() {
    let server = helpers::TestServer::start_with_world().await;
    let mut client = server.connect().await;

    client
        .register_and_login(
            &format!("user_{}", uuid::Uuid::new_v4().simple()),
            "password",
        )
        .await;

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

/// WRLD-02: Moving north from the default spawn room succeeds and returns
/// MoveOk followed by a RoomDescription of the new room.
#[tokio::test]
async fn test_move_cardinal_direction() {
    let server = helpers::TestServer::start_with_world().await;
    let mut client = server.connect().await;

    client
        .register_and_login(
            &format!("user_{}", uuid::Uuid::new_v4().simple()),
            "password",
        )
        .await;

    // Find starting room via look
    client.send_look().await;
    let start_desc = client.recv_world().await;
    let start_room_id = match &start_desc {
        WorldServerMsg::RoomDescription { room_id, .. } => room_id.clone(),
        other => panic!("expected RoomDescription after login+look, got {:?}", other),
    };

    // Move north (starting_village:market_square has a north exit to the tavern)
    client.send_move("north").await;

    let move_resp = client.recv_world().await;
    match &move_resp {
        WorldServerMsg::MoveOk { from_room, to_room } => {
            assert_eq!(from_room, &start_room_id, "from_room should match starting room");
            assert_ne!(to_room, &start_room_id, "to_room should differ from starting room");
        }
        other => panic!("expected MoveOk, got {:?}", other),
    }

    // Auto-look response after move
    let room_desc = client.recv_world().await;
    match room_desc {
        WorldServerMsg::RoomDescription { room_id, name, description, .. } => {
            assert_ne!(room_id, start_room_id, "new room ID should differ");
            assert!(!name.is_empty(), "new room name should not be empty");
            assert!(!description.is_empty(), "new room description should not be empty");
        }
        other => panic!("expected RoomDescription after move, got {:?}", other),
    }
}

/// WRLD-02: Single-letter direction alias 'n' behaves the same as 'north'.
#[tokio::test]
async fn test_move_alias() {
    let server = helpers::TestServer::start_with_world().await;
    let mut client = server.connect().await;

    client
        .register_and_login(
            &format!("user_{}", uuid::Uuid::new_v4().simple()),
            "password",
        )
        .await;

    // Send 'n' alias instead of 'north'
    client.send_move("n").await;

    let move_resp = client.recv_world().await;
    match move_resp {
        WorldServerMsg::MoveOk { .. } => {
            // success — alias worked
        }
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

    client
        .register_and_login(
            &format!("user_{}", uuid::Uuid::new_v4().simple()),
            "password",
        )
        .await;

    // The default spawn room (starting_village:market_square) has no 'up' exit
    client.send_move("up").await;

    let resp = client.recv_world().await;
    match resp {
        WorldServerMsg::MoveFail { .. } => {
            // correct — no exit in that direction
        }
        other => panic!("expected MoveFail for direction with no exit, got {:?}", other),
    }
}

// ── WRLD-03: Position persistence across restart ─────────────────────────────

/// WRLD-03: A player's position is preserved across server restarts.
/// Uses a file-based SQLite database shared between two TestServer instances.
#[tokio::test]
async fn test_position_survives_restart() {
    let db_path = tempfile::NamedTempFile::new()
        .expect("failed to create tempfile")
        .into_temp_path();
    let db_url = format!("sqlite:{}", db_path.to_str().unwrap());

    let username = format!("user_{}", uuid::Uuid::new_v4().simple());
    let password = "persist_test";

    // Server 1: login, move north, disconnect
    let new_room_id = {
        let server = helpers::TestServer::start_with_db(&db_url).await;
        let mut client = server.connect().await;
        client.register_and_login(&username, password).await;

        // Move north from spawn room
        client.send_move("north").await;
        let move_resp = client.recv_world().await;
        let new_room = match move_resp {
            WorldServerMsg::MoveOk { to_room, .. } => to_room,
            other => panic!("expected MoveOk, got {:?}", other),
        };
        let _desc = client.recv_world().await; // consume auto-look
        new_room
        // server and client drop here, simulating restart
    };

    // Small delay to ensure SQLite WAL is flushed
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Server 2: same DB, login again, look → should be in new_room_id
    let server2 = helpers::TestServer::start_with_db(&db_url).await;
    let mut client2 = server2.connect().await;

    // Login (no register — account already exists in DB)
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

    client2.send_look().await;
    let look_resp = client2.recv_world().await;
    match look_resp {
        WorldServerMsg::RoomDescription { room_id, .. } => {
            assert_eq!(
                room_id, new_room_id,
                "player should be in the room they moved to before restart"
            );
        }
        other => panic!("expected RoomDescription, got {:?}", other),
    }
}

// ── WRLD-04: Tutorial hints ──────────────────────────────────────────────────

/// WRLD-04: A new player in the newbie zone sees tutorial hints.
/// The default spawn is starting_village:market_square which has hints.
#[tokio::test]
async fn test_hints_shown_to_new_player() {
    let server = helpers::TestServer::start_with_world().await;
    let mut client = server.connect().await;

    client
        .register_and_login(
            &format!("user_{}", uuid::Uuid::new_v4().simple()),
            "password",
        )
        .await;

    client.send_look().await;
    let resp = client.recv_world().await;
    match resp {
        WorldServerMsg::RoomDescription { hints, .. } => {
            assert!(
                !hints.is_empty(),
                "new player should see tutorial hints in starting room (hints was empty)"
            );
        }
        other => panic!("expected RoomDescription, got {:?}", other),
    }
}

/// WRLD-04: After completing the tutorial, hints are suppressed even in rooms that have them.
///
/// This test navigates to the newbie garden and fires the 'pass through archway' trigger
/// which has a SetTutorialComplete effect (data-driven per D-11). Then it looks at a room
/// with hints and verifies the hints list is empty.
#[tokio::test]
async fn test_hints_suppressed_after_tutorial() {
    let server = helpers::TestServer::start_with_world().await;
    let mut client = server.connect().await;

    client
        .register_and_login(
            &format!("user_{}", uuid::Uuid::new_v4().simple()),
            "password",
        )
        .await;

    // Navigate from starting_village:market_square to newbie:garden
    // Path: market_square -> we need to reach the newbie zone
    // The newbie:garden has north -> starting_village:market_square
    // We need to go the other way: market_square has no direct link back to newbie zone
    // So we'll use a server that starts players in the newbie zone by overriding spawn.
    //
    // Alternative: start_with_world_and_spawn("newbie:entrance")
    // Since the default spawn is starting_village:market_square, we cannot navigate to
    // newbie:garden from there. We need a server variant that spawns in the newbie zone.

    let server2 = helpers::TestServer::start_with_spawn("newbie:entrance").await;
    let mut client2 = server2.connect().await;
    client2
        .register_and_login(
            &format!("user_{}", uuid::Uuid::new_v4().simple()),
            "password",
        )
        .await;

    // Navigate: entrance -> north -> courtyard -> west -> garden
    client2.send_move("north").await;
    let _ = client2.recv_world().await; // MoveOk
    let _ = client2.recv_world().await; // RoomDescription (courtyard)

    client2.send_move("west").await;
    let _ = client2.recv_world().await; // MoveOk
    let _ = client2.recv_world().await; // RoomDescription (garden with hints)

    // Fire the tutorial completion trigger
    client2.send_interact("pass through archway").await;
    let interact_resp = client2.recv_world().await;
    match &interact_resp {
        WorldServerMsg::InteractResult { text } => {
            assert!(text.contains("ready for the wider world") || !text.is_empty(),
                "tutorial completion trigger should return a message");
        }
        other => panic!("expected InteractResult, got {:?}", other),
    }

    // Now look in the garden (which has hints) — they should be suppressed
    client2.send_look().await;
    let look_resp = client2.recv_world().await;
    match look_resp {
        WorldServerMsg::RoomDescription { hints, room_id, .. } => {
            assert!(
                hints.is_empty(),
                "hints should be suppressed after tutorial completion (room: {})",
                room_id
            );
        }
        other => panic!("expected RoomDescription, got {:?}", other),
    }
}

// ── WRLD-05: Examine lore ────────────────────────────────────────────────────

/// WRLD-05: Examining a target with lore in the room returns the lore text.
#[tokio::test]
async fn test_examine_returns_lore() {
    let server = helpers::TestServer::start_with_world().await;
    let mut client = server.connect().await;

    client
        .register_and_login(
            &format!("user_{}", uuid::Uuid::new_v4().simple()),
            "password",
        )
        .await;

    // starting_village:market_square has lore containing "Aldric"
    // Examining "room" or "here" returns the room lore directly
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

    client
        .register_and_login(
            &format!("user_{}", uuid::Uuid::new_v4().simple()),
            "password",
        )
        .await;

    client.send_examine("nonexistent_purple_dragon_statue").await;
    let resp = client.recv_world().await;
    match resp {
        WorldServerMsg::ExamineResult { text } => {
            assert!(
                text.to_lowercase().contains("nothing of note"),
                "examining unknown target should return 'nothing of note', got: {:?}",
                text
            );
        }
        other => panic!("expected ExamineResult, got {:?}", other),
    }
}

// ── WRLD-06: Triggers ────────────────────────────────────────────────────────

/// WRLD-06: A trigger fires and its state change persists (second invocation
/// gets the "already done" response due to condition being false).
#[tokio::test]
async fn test_trigger_fires_and_persists() {
    let server = helpers::TestServer::start_with_spawn("newbie:practice_hall").await;
    let mut client = server.connect().await;

    client
        .register_and_login(
            &format!("user_{}", uuid::Uuid::new_v4().simple()),
            "password",
        )
        .await;

    // First pull: lever_state is absent (treated as "false"), trigger fires.
    // The trigger has 3 effects: set_state, reveal_exit, broadcast, message.
    // The actor sends: InteractResult (from message effect), then WorldEvent (from broadcast).
    client.send_interact("pull lever").await;
    let first_resp = client.recv_world().await;
    match &first_resp {
        WorldServerMsg::InteractResult { text } => {
            assert!(
                text.contains("lever") || text.contains("door") || text.contains("clunk"),
                "first pull should return the lever trigger message, got: {:?}",
                text
            );
        }
        other => panic!("expected InteractResult on first pull, got {:?}", other),
    }

    // Drain the WorldEvent broadcast from the first pull (the actor forwards it to self).
    let broadcast_event = client.recv_world().await;
    assert!(
        matches!(broadcast_event, WorldServerMsg::WorldEvent { .. }),
        "expected WorldEvent broadcast after first pull, got {:?}",
        broadcast_event
    );

    // Second pull: lever_state is now "true", first trigger's condition fails
    // (condition = "false" but state is now "true"), second trigger fires.
    client.send_interact("pull lever").await;
    let second_resp = client.recv_world().await;
    match &second_resp {
        WorldServerMsg::InteractResult { text } => {
            assert!(
                text.contains("already") || text.contains("lever"),
                "second pull should indicate lever is already pulled, got: {:?}",
                text
            );
        }
        other => panic!("expected InteractResult on second pull, got {:?}", other),
    }
}

/// WRLD-06: A trigger with a Broadcast effect delivers a WorldEvent to other
/// players in the same room.
#[tokio::test]
async fn test_trigger_broadcasts_to_room() {
    let server = helpers::TestServer::start_with_spawn("newbie:courtyard").await;

    // Connect two players to the same server (both will spawn in newbie:courtyard)
    let mut player_a = server.connect().await;
    let mut player_b = server.connect().await;

    let username_a = format!("player_a_{}", uuid::Uuid::new_v4().simple());
    let username_b = format!("player_b_{}", uuid::Uuid::new_v4().simple());

    player_a.register_and_login(&username_a, "password").await;
    player_b.register_and_login(&username_b, "password").await;

    // Player B: drain any pending broadcast from Player A's login arrival
    // (player arrival events may be sent to room subscribers)
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Player A fires the fountain trigger (first time, condition passes: fountain_touched absent = "false")
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
                message.contains(&username_a) || message.contains("fountain") || message.contains("glow"),
                "broadcast should reference the triggering player or the event, got: {:?}",
                message
            );
        }
        other => panic!("expected WorldEvent for Player B, got {:?}", other),
    }
}
