extern crate dotenv;
extern crate dotenv_codegen;
extern crate rusty_mvt;

use anyhow::{anyhow, Context, Error};
use axum::{
    routing::{get, post},
    Router,
};

use dotenv::dotenv;
use reqwest::{header::CONTENT_TYPE, Method};
use rusty_mvt::{
    db::{get_db_connector, load_table_registry, TableRegistry},
    geocoding::get_latlong,
    layers::get_layer,
    routing::get_circuit,
    AppState,
};

use sqlx::{Pool, Postgres};

use tower_http::cors::{Any, CorsLayer};

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

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any)
        .allow_headers([CONTENT_TYPE]);

    let app = Router::new()
        .route("/geocode/:queryString", get(get_latlong))
        .route("/layers/:schemaid/:tableid/:z/:x/:y_ext", get(get_layer))
        .route("/circuit/:schemaid/:tableid/", post(get_circuit))
        .with_state(state)
        .layer(cors);

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .context("Error occurred while starting server")
}
