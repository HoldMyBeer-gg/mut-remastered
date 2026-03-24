mod helpers;

use protocol::character;

/// AUTH-03: Multiple characters per account.
#[tokio::test]
async fn test_multiple_characters_per_account() {
    let server = helpers::TestServer::start_with_world().await;
    let mut client = server.connect().await;

    let username = format!("user_{}", uuid::Uuid::new_v4().simple());
    // Register + login
    client.send(&protocol::auth::ClientMsg::Register { username: username.clone(), password: "pass".to_string() }).await;
    let _ = client.recv().await;
    client.send(&protocol::auth::ClientMsg::Login { username: username.clone(), password: "pass".to_string() }).await;
    let _ = client.recv().await;

    // Create two characters (valid point buy = 27)
    // [8, 15, 12, 15, 10, 8] = 0+9+4+9+2+0 = 24... need 27
    // [8, 14, 13, 15, 12, 8] = 0+7+5+9+4+0 = 25... 
    // [10, 14, 13, 15, 12, 8] = 2+7+5+9+4+0 = 27 ✓
    client.send_char(&character::ClientMsg::CharacterCreate {
        name: "Hero One".to_string(), race: "elf".to_string(), class: "mage".to_string(),
        gender: "female".to_string(), ability_scores: [10, 14, 13, 15, 12, 8],
        racial_bonus_choices: vec![],
    }).await;
    let resp1 = client.recv_char().await;
    assert!(matches!(resp1, character::ServerMsg::CharacterCreateOk { .. }));

    client.send_char(&character::ClientMsg::CharacterCreate {
        name: "Hero Two".to_string(), race: "dwarf".to_string(), class: "warrior".to_string(),
        gender: "male".to_string(), ability_scores: [15, 14, 13, 12, 10, 8],
        racial_bonus_choices: vec![],
    }).await;
    let resp2 = client.recv_char().await;
    assert!(matches!(resp2, character::ServerMsg::CharacterCreateOk { .. }));

    // List should show 2 characters
    client.send_char(&character::ClientMsg::CharacterList).await;
    let list = client.recv_char().await;
    match list {
        character::ServerMsg::CharacterListResult { characters } => {
            assert_eq!(characters.len(), 2, "should have 2 characters");
        }
        other => panic!("expected CharacterListResult, got {:?}", other),
    }
}

/// AUTH-04: Invalid race is rejected.
#[tokio::test]
async fn test_invalid_race_rejected() {
    let server = helpers::TestServer::start_with_world().await;
    let mut client = server.connect().await;

    let username = format!("user_{}", uuid::Uuid::new_v4().simple());
    client.send(&protocol::auth::ClientMsg::Register { username: username.clone(), password: "pass".to_string() }).await;
    let _ = client.recv().await;
    client.send(&protocol::auth::ClientMsg::Login { username: username.clone(), password: "pass".to_string() }).await;
    let _ = client.recv().await;

    client.send_char(&character::ClientMsg::CharacterCreate {
        name: "Bad Race".to_string(), race: "dragon".to_string(), class: "warrior".to_string(),
        gender: "male".to_string(), ability_scores: [15, 14, 13, 12, 10, 8],
        racial_bonus_choices: vec![],
    }).await;
    let resp = client.recv_char().await;
    assert!(matches!(resp, character::ServerMsg::CharacterCreateFail { .. }), "expected CharacterCreateFail, got {:?}", resp);
}

/// AUTH-06: Invalid point buy is rejected.
#[tokio::test]
async fn test_invalid_point_buy_rejected() {
    let server = helpers::TestServer::start_with_world().await;
    let mut client = server.connect().await;

    let username = format!("user_{}", uuid::Uuid::new_v4().simple());
    client.send(&protocol::auth::ClientMsg::Register { username: username.clone(), password: "pass".to_string() }).await;
    let _ = client.recv().await;
    client.send(&protocol::auth::ClientMsg::Login { username: username.clone(), password: "pass".to_string() }).await;
    let _ = client.recv().await;

    // All 15s = 9*6 = 54 points, way over 27
    client.send_char(&character::ClientMsg::CharacterCreate {
        name: "Overpowered".to_string(), race: "human".to_string(), class: "warrior".to_string(),
        gender: "male".to_string(), ability_scores: [15, 15, 15, 15, 15, 15],
        racial_bonus_choices: vec![0, 1],
    }).await;
    let resp = client.recv_char().await;
    assert!(matches!(resp, character::ServerMsg::CharacterCreateFail { .. }), "expected CharacterCreateFail, got {:?}", resp);
}

/// CHAR-01: Vitals sent after character select.
#[tokio::test]
async fn test_vitals_on_character_select() {
    let server = helpers::TestServer::start_with_world().await;
    let mut client = server.connect().await;

    let username = format!("user_{}", uuid::Uuid::new_v4().simple());
    // Register + login
    client.send(&protocol::auth::ClientMsg::Register { username: username.clone(), password: "pass".to_string() }).await;
    let _ = client.recv().await;
    client.send(&protocol::auth::ClientMsg::Login { username: username.clone(), password: "pass".to_string() }).await;
    let _ = client.recv().await;

    // Create character (valid 27-point buy)
    client.send_char(&character::ClientMsg::CharacterCreate {
        name: "VitalCheck".to_string(), race: "orc".to_string(), class: "warrior".to_string(),
        gender: "male".to_string(), ability_scores: [15, 14, 13, 12, 10, 8],
        racial_bonus_choices: vec![],
    }).await;
    let create_resp = client.recv_char().await;
    let char_id = match create_resp {
        character::ServerMsg::CharacterCreateOk { character_id, .. } => character_id,
        other => panic!("expected CharacterCreateOk, got {:?}", other),
    };

    // Select character
    client.send_char(&character::ClientMsg::CharacterSelect { character_id: char_id }).await;
    let _selected = client.recv_char().await; // CharacterSelected
    let _room = client.recv_world().await;     // RoomDescription
    let vitals = client.recv_combat().await;   // Vitals

    match vitals {
        protocol::combat::ServerMsg::Vitals { hp, max_hp, .. } => {
            assert!(hp > 0, "HP should be positive");
            assert_eq!(hp, max_hp, "HP should equal max_hp for a new character");
        }
        other => panic!("expected Vitals, got {:?}", other),
    }
}

/// CHAR-05: Biography can be set and read.
#[tokio::test]
async fn test_character_bio() {
    let server = helpers::TestServer::start_with_world().await;
    let mut client = server.connect().await;

    let username = format!("user_{}", uuid::Uuid::new_v4().simple());
    client.full_setup(&username, "pass", "BioHero").await;

    // Set bio
    client.send_world(&protocol::world::ClientMsg::Bio {
        text: "A wanderer from distant lands.".to_string(),
    }).await;
    let bio_resp = client.recv_world().await;
    assert!(matches!(bio_resp, protocol::world::ServerMsg::BioOk), "expected BioOk, got {:?}", bio_resp);

    // Verify via Stats
    client.send_world(&protocol::world::ClientMsg::Stats).await;
    let stats = client.recv_world().await;
    match stats {
        protocol::world::ServerMsg::StatsResult { bio, name, .. } => {
            assert_eq!(name, "BioHero");
            assert_eq!(bio, "A wanderer from distant lands.");
        }
        other => panic!("expected StatsResult, got {:?}", other),
    }
}
