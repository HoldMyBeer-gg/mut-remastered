use sqlx::SqlitePool;
use uuid::Uuid;

/// Create a new session for the given account, returning the session token.
///
/// The token is a UUID v4 string. Expiry is set to `unixepoch() + ttl_secs`.
pub async fn create_session(
    pool: &SqlitePool,
    account_id: &str,
    ttl_secs: i64,
) -> anyhow::Result<String> {
    let token = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO sessions (token, account_id, created_at, expires_at) \
         VALUES (?1, ?2, unixepoch(), unixepoch() + ?3)",
    )
    .bind(&token)
    .bind(account_id)
    .bind(ttl_secs)
    .execute(pool)
    .await?;
    Ok(token)
}

/// Validate a session token.
///
/// Returns `Some(account_id)` if the token exists and has not expired.
/// Returns `None` if the token is not found or expired.
pub async fn validate_session(pool: &SqlitePool, token: &str) -> anyhow::Result<Option<String>> {
    let row = sqlx::query_as::<_, (String,)>(
        "SELECT account_id FROM sessions WHERE token = ?1 AND expires_at > unixepoch()",
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(account_id,)| account_id))
}

/// Delete a session by token. Used for explicit logout (AUTH-08).
pub async fn delete_session(pool: &SqlitePool, token: &str) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM sessions WHERE token = ?1")
        .bind(token)
        .execute(pool)
        .await?;
    Ok(())
}

/// Register a new account, returning the generated account id.
///
/// Returns an error with a descriptive message if the username is already taken
/// (UNIQUE constraint violation on the accounts table).
pub async fn register_account(
    pool: &SqlitePool,
    username: &str,
    password_hash: &str,
) -> anyhow::Result<String> {
    let account_id = Uuid::new_v4().to_string();
    let result = sqlx::query(
        "INSERT INTO accounts (id, username, password_hash, created_at) \
         VALUES (?1, ?2, ?3, unixepoch())",
    )
    .bind(&account_id)
    .bind(username)
    .bind(password_hash)
    .execute(pool)
    .await;

    match result {
        Ok(_) => Ok(account_id),
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            Err(anyhow::anyhow!("username '{}' is already taken", username))
        }
        Err(e) => Err(anyhow::Error::from(e)),
    }
}

/// Look up an account by username.
///
/// Returns `Some((account_id, password_hash))` if found, `None` if not.
pub async fn lookup_account(
    pool: &SqlitePool,
    username: &str,
) -> anyhow::Result<Option<(String, String)>> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT id, password_hash FROM accounts WHERE username = ?1 COLLATE NOCASE",
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
