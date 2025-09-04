pub mod models;

use sqlx::{SqlitePool, migrate::MigrateDatabase, Sqlite};
use anyhow::Result;

pub async fn initialize_database() -> Result<SqlitePool> {
    let database_url = "sqlite:timekpr.db";
    
    // Create database file if it doesn't exist
    if !Sqlite::database_exists(database_url).await.unwrap_or(false) {
        Sqlite::create_database(database_url).await?;
        tracing::info!("Created database file");
    }
    
    let pool = SqlitePool::connect(database_url).await?;
    
    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;
    
    tracing::info!("Database initialized successfully");
    Ok(pool)
}