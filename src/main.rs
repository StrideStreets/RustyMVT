use axum::{routing::get, Router};

mod geocoding;
use geocoding::get_latlong;

mod layers;
use layers::get_layer;

mod api;
use api::serve_tile;

#[macro_use]
extern crate dotenv_codegen;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/geocode/:queryString", get(get_latlong))
        .route("/layers/:tableid/:x/:y/:z.mvt", get(get_layer))
        .route("/api/:schema/:table/:x/:y/:z:ext", get(serve_tile));

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
