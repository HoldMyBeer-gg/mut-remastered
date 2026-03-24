/// Server configuration. Loaded from environment variables with sensible defaults.
pub struct ServerConfig {
    pub bind_addr: String,
    pub database_url: String,
    pub session_ttl_secs: i64,
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
        }
    }
}
