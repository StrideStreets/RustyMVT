mod table_specs;

use anyhow::{anyhow, bail};
use axum::{
    body::{Bytes, Full},
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
};

use axum_macros::debug_handler;
use reqwest::header;

use crate::{
    db::structs::Table, structs::Tile, transformations::vector_tile::make_tile_data_query,
    AppError, AppState,
};

use sqlx::{query, Pool, Postgres, Row};

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

    let y_ext_parts: Vec<&str> = y_ext.split('.').into_iter().collect();
    let y: usize = y_ext_parts[0]
        .parse()
        .expect("Y should be parsable as usize");
    let ext = y_ext_parts[1];

    let this_tile = Tile::new(x, y, z);

    match ext {
        "mvt" => get_mvt(&this_tile, table_spec, state.db_pool)
            .await
            .and_then(|mvt_body| {
                let mut headers = HeaderMap::new();
                headers.insert(
                    "Content-Type",
                    "application/vnd.mapbox-vector-tile".parse().unwrap(),
                );
                Ok((headers, mvt_body))
            }),
        _ => return Err(AppError(anyhow!("Specified file extension not supported"))), //Or something -- this is meant to indicate that the format is not currently supported,
    }
}

async fn get_mvt(tile: &Tile, table: &Table, conn: Pool<Postgres>) -> Result<Vec<u8>, AppError> {
    if let Ok(mvt_query) = make_tile_data_query(tile, table) {
        println!("{}", &mvt_query);
        match query(&mvt_query).fetch_all(&conn).await {
            Ok(mvt_result) => {
                let mvt_bytes: Vec<u8> = mvt_result[0].get(0);
                return Ok(mvt_bytes);
            }
            Err(e) => {
                println!("{:?}", e);
                return Err(AppError(anyhow!(format!(
                    "Failed to locate specified table. Received error {}",
                    e
                ))));
            }
        }
    } else {
        return Err(AppError(anyhow!(
            "Failed to assemble MVT query from provided parameters"
        )));
    }
}
