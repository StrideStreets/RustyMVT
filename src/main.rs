#[macro_use]
extern crate dotenv_codegen;
extern crate dotenv;

use anyhow::Context;
use axum::{routing::get, Router};
use dotenv::dotenv;

mod geocoding;
use geocoding::get_latlong;

mod layers;
use layers::get_layer;

mod api;
use api::serve_tile;

mod db;
use db::{get_db_connector, load_table_registry, structs::TableRegistry};
use sqlx::{Pool, Postgres};

#[derive(Clone)]
pub struct AppState {
    db_pool: Pool<Postgres>,
    table_registry: TableRegistry,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let db_pool = get_db_connector()
        .await
        .context("Failed to get database connector)")
        .unwrap();

    let table_registry = load_table_registry(&db_pool, "default".to_string())
        .await
        .context("Failed to load table registry")
        .unwrap();

    let state = AppState {
        db_pool,
        table_registry,
    };

    let app = Router::new()
        .route("/geocode/:queryString", get(get_latlong))
        .route("/layers/:tableid/:x/:y/:z.mvt", get(get_layer))
        .route("/api/:schema/:table/:x/:y/:z", get(serve_tile))
        .with_state(state);

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
