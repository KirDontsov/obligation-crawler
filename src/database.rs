use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;
use std::time::Duration;

pub async fn create_connection_pool() -> Result<PgPool, sqlx::Error> {
    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL must be set in environment variables");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .test_before_acquire(false)
        .connect(&database_url)
        .await?;

    test_connection(&pool).await?;

    Ok(pool)
}

async fn test_connection(pool: &PgPool) -> Result<(), sqlx::Error> {
    match sqlx::query("SELECT 1").fetch_one(pool).await {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("❌ Database connection test failed: {}", e);
            Err(e)
        }
    }
}