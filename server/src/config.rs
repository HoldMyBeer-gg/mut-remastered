/// Server configuration. Loaded from environment variables with sensible defaults.
pub struct ServerConfig {
    pub bind_addr: String,
    pub database_url: String,
    pub session_ttl_secs: i64,
    /// Path to the directory containing zone subdirectories (each with a zone.toml).
    pub worlds_dir: String,
    /// WebSocket server bind address (for browser clients).
    pub ws_bind_addr: String,
}

impl ServerConfig {
    pub fn from_env() -> Self {
        Self {
            bind_addr: std::env::var("BIND_ADDR")
                .unwrap_or_else(|_| "127.0.0.1:4000".to_string()),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://./mut_remastered.db?mode=rwc".to_string()),
            session_ttl_secs: std::env::var("SESSION_TTL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(604800), // 7 days
            worlds_dir: std::env::var("MUT_WORLDS_DIR")
                .unwrap_or_else(|_| "../world/zones".to_string()),
            ws_bind_addr: std::env::var("WS_BIND_ADDR")
                .unwrap_or_else(|_| "127.0.0.1:4001".to_string()),
        }
    }
}
