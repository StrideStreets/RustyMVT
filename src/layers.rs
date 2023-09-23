mod table_specs;

use axum::{
    extract::{Path, State},
    Error,
};
use axum_macros::debug_handler;
use reqwest::StatusCode;

use crate::AppState;

#[derive(Debug)]
pub struct Tile {
    z: usize,
    x: usize,
    y: usize,
}

impl Tile {
    fn new(x: usize, y: usize, z: usize) -> Self {
        Tile { x, y, z }
    }
}

#[debug_handler]
pub async fn get_layer(
    State(state): State<AppState>,
    Path((schemaid, tableid, z, x, y_ext)): Path<(String, String, usize, usize, String)>,
) -> Result<(), StatusCode> {
    let table_spec;
    if let Some(schema) = state.table_registry.schemas.get(&schemaid) {
        if let Some(table) = schema.tables.get(&tableid) {
            table_spec = table;
        } else {
            return Err(StatusCode::BAD_REQUEST);
        }
    } else {
        return Err(StatusCode::BAD_REQUEST);
    }

    let y_ext_parts: Vec<&str> = y_ext.split('.').into_iter().collect();
    let y: usize = y_ext_parts[0]
        .parse()
        .expect("Y should be parsable as usize");
    let ext = y_ext_parts[1];

    let this_tile = Tile::new(x, y, z);

    Ok(())
}
