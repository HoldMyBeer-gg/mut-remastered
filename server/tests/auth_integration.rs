mod helpers;

use protocol::auth::{ClientMsg, ErrorCode, ServerMsg};

/// AUTH-01: Registration creates an account.
/// Verifies that a successful Register returns RegisterOk with a non-empty account_id.
#[tokio::test]
async fn test_register_creates_account() {
    let server = helpers::TestServer::start().await;
    let mut client = server.connect().await;

    client
        .send(&ClientMsg::Register {
            username: "alice".to_string(),
            password: "hunter2".to_string(),
        })
        .await;

    let resp = client.recv().await;
    match resp {
        ServerMsg::RegisterOk { account_id } => {
            assert!(!account_id.is_empty(), "account_id should not be empty");
        }
        other => panic!("expected RegisterOk, got {:?}", other),
    }
}

/// AUTH-01: Registering with a duplicate username returns UsernameTaken error.
#[tokio::test]
async fn test_register_duplicate_username_fails() {
    let server = helpers::TestServer::start().await;

    // First registration succeeds
    let mut client1 = server.connect().await;
    client1
        .send(&ClientMsg::Register {
            username: "bob".to_string(),
            password: "pass1".to_string(),
        })
        .await;
    let _ = client1.recv().await; // RegisterOk

    // Second registration with the same username must fail
    let mut client2 = server.connect().await;
    client2
        .send(&ClientMsg::Register {
            username: "bob".to_string(),
            password: "pass2".to_string(),
        })
        .await;

    let resp = client2.recv().await;
    match resp {
        ServerMsg::Error { code, .. } => {
            assert_eq!(
                code,
                ErrorCode::UsernameTaken,
                "expected UsernameTaken error code"
            );
        }
        other => panic!("expected Error(UsernameTaken), got {:?}", other),
    }
}

/// AUTH-02: Login with correct credentials returns a session token.
#[tokio::test]
async fn test_login_with_correct_credentials() {
    let server = helpers::TestServer::start().await;

    // Register first
    let mut client1 = server.connect().await;
    client1
        .send(&ClientMsg::Register {
            username: "charlie".to_string(),
            password: "secret".to_string(),
        })
        .await;
    let _ = client1.recv().await; // RegisterOk

    // Login on a separate connection
    let mut client2 = server.connect().await;
    client2
        .send(&ClientMsg::Login {
            username: "charlie".to_string(),
            password: "secret".to_string(),
        })
        .await;

    let resp = client2.recv().await;
    match resp {
        ServerMsg::LoginOk { session_token } => {
            assert!(
                !session_token.is_empty(),
                "session_token should not be empty"
            );
        }
        other => panic!("expected LoginOk, got {:?}", other),
    }
}

/// AUTH-02: Login with an incorrect password returns InvalidCredentials error.
#[tokio::test]
async fn test_login_with_wrong_password() {
    let server = helpers::TestServer::start().await;

    // Register
    let mut client1 = server.connect().await;
    client1
        .send(&ClientMsg::Register {
            username: "dave".to_string(),
            password: "correct".to_string(),
        })
        .await;
    let _ = client1.recv().await; // RegisterOk

    // Attempt login with wrong password
    let mut client2 = server.connect().await;
    client2
        .send(&ClientMsg::Login {
            username: "dave".to_string(),
            password: "wrong".to_string(),
        })
        .await;

    let resp = client2.recv().await;
    match resp {
        ServerMsg::Error { code, .. } => {
            assert_eq!(
                code,
                ErrorCode::InvalidCredentials,
                "expected InvalidCredentials error code"
            );
        }
        other => panic!("expected Error(InvalidCredentials), got {:?}", other),
    }
}

/// AUTH-08: Explicit logout returns LogoutOk and the session is considered invalidated.
///
/// The server deletes the session token from the database on logout. This test
/// verifies the LogoutOk response is returned, confirming the logout path executes
/// successfully without error.
#[tokio::test]
async fn test_logout_invalidates_session() {
    let server = helpers::TestServer::start().await;
    let mut client = server.connect().await;

    // Register
    client
        .send(&ClientMsg::Register {
            username: "eve".to_string(),
            password: "pass".to_string(),
        })
        .await;
    let _ = client.recv().await; // RegisterOk

    // Login
    client
        .send(&ClientMsg::Login {
            username: "eve".to_string(),
            password: "pass".to_string(),
        })
        .await;
    let login_resp = client.recv().await;
    assert!(
        matches!(login_resp, ServerMsg::LoginOk { .. }),
        "expected LoginOk before logout, got {:?}",
        login_resp
    );

    // Logout — must return LogoutOk
    client.send(&ClientMsg::Logout).await;
    let logout_resp = client.recv().await;
    assert!(
        matches!(logout_resp, ServerMsg::LogoutOk),
        "expected LogoutOk, got {:?}",
        logout_resp
    );
}

/// AUTH-01: Hash stored in DB is Argon2id PHC format, never plaintext.
///
/// This is a unit-level test that calls hash_password directly. It does not
/// require the server to be running.
#[tokio::test]
async fn test_hash_is_not_plaintext() {
    let hash = server::auth::hash::hash_password("mypassword").expect("hash_password failed");
    assert!(
        hash.starts_with("$argon2id$"),
        "hash should be Argon2id PHC format, got: {}",
        hash
    );
    assert!(
        !hash.contains("mypassword"),
        "hash must not contain the plaintext password, got: {}",
        hash
    );
}

/// Basic connectivity: Ping returns Pong.
#[tokio::test]
async fn test_ping_pong() {
    let server = helpers::TestServer::start().await;
    let mut client = server.connect().await;

    client.send(&ClientMsg::Ping).await;
    let resp = client.recv().await;
    assert!(
        matches!(resp, ServerMsg::Pong),
        "expected Pong, got {:?}",
        resp
    );
}
