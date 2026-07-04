use sea_orm::{DatabaseConnection, EntityTrait, PaginatorTrait, SqlxSqliteConnector};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::str::FromStr;
use std::time::Duration;

use crate::entity::user as user_entity;
use crate::seed;
use migration::{Migrator, MigratorTrait};

/// Initialise the database connection, run migrations, and seed default data.
pub async fn init(database_url: &str) -> DatabaseConnection {
    let db = connect(database_url)
        .await
        .expect("Failed to connect to database");

    Migrator::up(&db, None)
        .await
        .expect("Failed to run migrations");

    seed::seed_accounts(&db).await;
    seed::seed_claim_categories(&db).await;

    db
}

/// Check user count in database
pub async fn has_no_users(db: &DatabaseConnection) -> bool {
    match user_entity::Entity::find().count(db).await {
        Ok(n) => n == 0,
        Err(e) => {
            eprintln!("warning: failed to count users: {e}; assuming empty");
            true
        }
    }
}

/// Create a connection pool with WAL mode and foreign keys enabled for concurrency.
async fn connect(url: &str) -> Result<DatabaseConnection, sea_orm::DbErr> {
    let opts = SqliteConnectOptions::from_str(url)
        .map_err(|e| sea_orm::DbErr::Custom(e.to_string()))?
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);

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
