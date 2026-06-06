use sea_orm::{DatabaseConnection, SqlxSqliteConnector};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::str::FromStr;
use std::time::Duration;

use crate::seed;
use migration::{Migrator, MigratorTrait};

pub async fn init(database_url: &str) -> DatabaseConnection {
    let db = connect(database_url)
        .await
        .expect("Failed to connect to database");

    Migrator::up(&db, None)
        .await
        .expect("Failed to run migrations");

    seed::seed(&db).await;

    db
}

async fn connect(url: &str) -> Result<DatabaseConnection, sea_orm::DbErr> {
    // Enable WAL mode for concurrency
    let opts = SqliteConnectOptions::from_str(url)
        .map_err(|e| sea_orm::DbErr::Custom(e.to_string()))?
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);

    // Create a connection pool with reasonable defaults
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(600))
        .connect_with(opts)
        .await
        .map_err(|e| sea_orm::DbErr::Custom(e.to_string()))?;

    Ok(SqlxSqliteConnector::from_sqlx_sqlite_pool(pool))
}
