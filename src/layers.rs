mod vector_tile;
use anyhow::anyhow;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
};
use vector_tile::{get_mvt, Tile};

use axum_macros::debug_handler;

use crate::{AppError, AppState};

#[debug_handler]
pub async fn get_layer(
    State(state): State<AppState>,
    Path((schemaid, tableid, z, x, y_ext)): Path<(String, String, usize, usize, String)>,
) -> Result<(HeaderMap, impl IntoResponse), AppError> {
    let table_spec;
    if let Some(schema) = state.table_registry.schemas.get(&schemaid) {
        if let Some(table) = schema.tables.get(&tableid) {
            table_spec = table;
        } else {
            return Err(AppError(anyhow!("Failed to locate specified table")));
        }
    } else {
        return Err(AppError(anyhow!("Failed to locate specified schema")));
    }

    let y_ext_parts: Vec<&str> = y_ext.split('.').collect();
    let y: usize = y_ext_parts[0]
        .parse()
        .expect("Y should be parsable as usize");
    let ext = y_ext_parts[1];

    let this_tile = Tile::new(x, y, z);

    match ext {
        "mvt" => get_mvt(&this_tile, table_spec, state.db_pool)
            .await
            .map(|mvt_body| {
                let mut headers = HeaderMap::new();
                headers.insert(
                    "Content-Type",
                    "application/vnd.mapbox-vector-tile".parse().unwrap(),
                );
                (headers, mvt_body)
            }),
        _ => Err(AppError(anyhow!("Specified file extension not supported"))), //Or something -- this is meant to indicate that the format is not currently supported,
    }
}
