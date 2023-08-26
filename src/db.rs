use anyhow::{Context, Result};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::env::var;

pub async fn get_db_connector() -> Result<Pool<Postgres>> {
    let DB_URL =
        var("DB_CONNECTION_STRING").context("No connection string found in environment")?;
    let USER = var("DB_USER")
        .context("No DB_USER var found in environment")
        .unwrap();
    let PW = var("DB_PW")
        .context("No DB_PW var found in environment")
        .unwrap();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&DB_URL)
        .await
        .context("Failed to connect using provided connection string.")
        .unwrap();

    return Ok(pool);
}

//Info on making queries here: https://github.com/launchbadge/sqlx#usage
