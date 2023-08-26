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
use db::get_db_connector;
use sqlx::{Pool, Postgres};

#[derive(Clone)]
pub struct AppState {
    db_pool: Pool<Postgres>,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let db_pool = get_db_connector()
        .await
        .context("Failed to get database connector)")
        .unwrap();

    let state = AppState { db_pool };

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
