mod helpers;

use protocol::auth::{ClientMsg, ServerMsg};
use tokio::task::JoinSet;

/// NETW-01: 10 concurrent connections can register and login independently.
///
/// Spawns 10 Tokio tasks simultaneously, each on its own TCP connection.
/// Every task registers a unique account and then logs in. All 10 must
/// complete successfully, proving the accept loop and per-connection actors
/// do not block each other.
#[tokio::test]
async fn test_10_concurrent_connections() {
    let server = helpers::TestServer::start().await;

    let mut join_set = JoinSet::new();

    for i in 0..10u32 {
        let addr = server.addr.clone();
        join_set.spawn(async move {
            let stream = tokio::net::TcpStream::connect(&addr)
                .await
                .unwrap_or_else(|e| panic!("user_{} failed to connect: {}", i, e));
            let mut client = helpers::TestClient::from_stream(stream);

            let username = format!("concurrent_user_{}", i);
            let password = format!("concurrent_pass_{}", i);

            // Register
            client
                .send(&ClientMsg::Register {
                    username: username.clone(),
                    password: password.clone(),
                })
                .await;
            let resp = client.recv().await;
            assert!(
                matches!(resp, ServerMsg::RegisterOk { .. }),
                "user_{} register failed: {:?}",
                i,
                resp
            );

            // Login
            client.send(&ClientMsg::Login { username, password }).await;
            let resp = client.recv().await;
            assert!(
                matches!(resp, ServerMsg::LoginOk { .. }),
                "user_{} login failed: {:?}",
                i,
                resp
            );

            i // Return index to confirm this task completed
        });
    }

    let mut completed = Vec::new();
    while let Some(result) = join_set.join_next().await {
        let idx = result.expect("concurrent test task panicked");
        completed.push(idx);
    }

    assert_eq!(
        completed.len(),
        10,
        "expected all 10 concurrent connections to complete, got {}",
        completed.len()
    );
}
