use sqlx::{
    Error, Pool, Sqlite,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteRow},
};

#[derive(sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub role: String,
}

pub async fn fetch_users(pool: &Pool<Sqlite>) -> Result<Vec<User>, Error> {
    sqlx::query_as::<_, User>("SELECT id, name, email, role FROM users")
        .fetch_all(&*pool)
        .await
}

pub async fn fetch_users2(pool: &Pool<Sqlite>) -> Result<Vec<SqliteRow>, Error> {
    sqlx::query("SELECT id, name, email, role FROM users")
    .fetch_all(&*pool)
    .await
}

pub async fn create_db() -> Result<Pool<Sqlite>, Error> {
    let db_url = "accounting.db";
    let db_exist = std::path::Path::new(db_url).exists();

    let options = SqliteConnectOptions::new()
        .filename(db_url)
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    if !db_exist {
        println!("Database created at {}", db_url);
        sqlx::migrate!().run(&pool).await?;
    }
    Ok(pool)
}
