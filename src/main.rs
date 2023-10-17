extern crate dotenv;
extern crate dotenv_codegen;
extern crate rusty_mvt;

use std::time::Duration;

use anyhow::{anyhow, Context, Error};
use axum::{
    error_handling::HandleErrorLayer,
    http::StatusCode,
    routing::{get, post},
    BoxError, Router,
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

use tower::ServiceBuilder;

async fn handle_timeout_error(err: BoxError) -> (StatusCode, String) {
    if err.is::<tower::timeout::error::Elapsed>() {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Request took too long".to_string(),
        )
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unhandled internal error: {}", err),
        )
    }
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

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any)
        .allow_headers([CONTENT_TYPE]);

    let timeout = ServiceBuilder::new()
        // `timeout` will produce an error if the handler takes
        // too long so we must handle those
        .layer(HandleErrorLayer::new(handle_timeout_error))
        .timeout(Duration::from_secs(5));

    let app = Router::new()
        .route("/geocode/:queryString", get(get_latlong))
        .route("/layers/:schemaid/:tableid/:z/:x/:y_ext", get(get_layer))
        .route("/circuit/:schemaid/:tableid/", post(get_circuit))
        .with_state(state)
        .layer(timeout)
        .layer(cors);

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .context("Error occurred while starting server")
}
