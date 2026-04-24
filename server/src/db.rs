use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

pub async fn init_db(database_url: &str) -> anyhow::Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    // Enable WAL mode for concurrent reads
    sqlx::query("PRAGMA journal_mode=WAL")
        .execute(&pool)
        .await?;

    // Enable foreign keys
    sqlx::query("PRAGMA foreign_keys=ON").execute(&pool).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("database initialized and migrations applied");
    Ok(pool)
}
