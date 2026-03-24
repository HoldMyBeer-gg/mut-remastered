use protocol::auth::{ClientMsg, ServerMsg};
use protocol::codec::{decode_message, encode_message};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// A test server started on a random OS-assigned port backed by a temporary in-memory SQLite DB.
///
/// The server runs in a background Tokio task for the lifetime of this struct.
/// Because each `TestServer` uses a unique in-memory database, tests run concurrently
/// without interfering with each other's account/session state.
pub struct TestServer {
    pub addr: String,
    /// Keep handle alive so the server task continues running.
    _handle: tokio::task::JoinHandle<()>,
}

impl TestServer {
    /// Start a real server on a random port with an isolated in-memory database.
    pub async fn start() -> Self {
        // Use a unique in-memory SQLite database per test to avoid state bleeding between
        // concurrently running test cases. The `?mode=memory&cache=shared` variant requires
        // a name, so we use a UUID to make it unique.
        let db_url = format!(
            "sqlite:file:testdb_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().simple()
        );

        let pool = server::db::init_db(&db_url)
            .await
            .expect("failed to init test database");

        let state = server::net::listener::AppState {
            db: pool,
            session_ttl_secs: 3600,
        };

        // Bind to port 0 to get an OS-assigned free port, record the address,
        // then drop the listener so run_listener can rebind to the same address.
        // There is a tiny race window but it is acceptable in local test environments.
        let probe = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind probe listener");
        let addr = probe.local_addr().expect("get probe addr").to_string();
        drop(probe);

        let addr_clone = addr.clone();
        let handle = tokio::spawn(async move {
            server::net::listener::run_listener(&addr_clone, state)
                .await
                .expect("server task failed");
        });

        // Allow the server task a moment to bind and enter the accept loop.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        TestServer {
            addr,
            _handle: handle,
        }
    }

    /// Open a new TCP connection to this test server.
    pub async fn connect(&self) -> TestClient {
        let stream = TcpStream::connect(&self.addr)
            .await
            .expect("failed to connect to test server");
        TestClient { stream }
    }
}

/// A connected test client that wraps a `TcpStream` and provides typed send/recv helpers.
pub struct TestClient {
    stream: TcpStream,
}

impl TestClient {
    /// Construct a `TestClient` from an already-connected `TcpStream`.
    ///
    /// Used by the concurrent connections test where streams are created independently.
    pub fn from_stream(stream: TcpStream) -> Self {
        TestClient { stream }
    }

    /// Encode and send a `ClientMsg` to the server.
    pub async fn send(&mut self, msg: &ClientMsg) {
        let bytes = encode_message(msg).expect("failed to encode ClientMsg");
        self.stream
            .write_all(&bytes)
            .await
            .expect("failed to write to test server");
    }

    /// Read the next length-prefixed frame from the server and decode it as a `ServerMsg`.
    pub async fn recv(&mut self) -> ServerMsg {
        // Read 4-byte LE length prefix
        let mut len_buf = [0u8; 4];
        self.stream
            .read_exact(&mut len_buf)
            .await
            .expect("failed to read length prefix from test server");
        let payload_len = u32::from_le_bytes(len_buf) as usize;

        // Read payload
        let mut payload = vec![0u8; payload_len];
        self.stream
            .read_exact(&mut payload)
            .await
            .expect("failed to read payload from test server");

        decode_message::<ServerMsg>(&payload).expect("failed to decode ServerMsg")
    }
}
