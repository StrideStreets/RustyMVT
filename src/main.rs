#[macro_use]
extern crate dotenv_codegen;
extern crate dotenv;

use anyhow::{anyhow, Context, Error};
use axum::{
    response::{IntoResponse, Response},
    routing::get,
    Router,
};

use dotenv::dotenv;

mod geocoding;
use geocoding::get_latlong;

mod layers;
use layers::get_layer;

use reqwest::StatusCode;

mod db;
use db::{get_db_connector, load_table_registry, TableRegistry};
use sqlx::{Pool, Postgres};

pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[derive(Clone)]
pub struct AppState {
    db_pool: Pool<Postgres>,
    table_registry: TableRegistry,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv().ok();

    let table_registry: TableRegistry;
    let db_pool: Pool<Postgres>;

    if let Ok(pool) = get_db_connector().await {
        db_pool = pool;
        if let Ok(registry) = load_table_registry(&db_pool, "default".to_string()).await {
            table_registry = registry;
        } else {
            return Err(anyhow!("Failed to load table registry"));
        };
    } else {
        return Err(anyhow!("Failed to connect with provided database string"));
    };

    let state = AppState {
        db_pool,
        table_registry,
    };

    let app = Router::new()
        .route("/geocode/:queryString", get(get_latlong))
        .route("/layers/:schemaid/:tableid/:z/:x/:y_ext", get(get_layer))
        //.route("/api/:schema/:table/:z/:x/:y", get(serve_tile))
        .with_state(state);

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .context("Error occurred while starting server")
}
