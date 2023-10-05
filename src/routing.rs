mod utils;
use crate::{AppError, AppState, MILES_TO_MINUTES_FACTOR};
use anyhow::{anyhow, bail};
use axum::{
    body::Body,
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use axum_macros::debug_handler;
use geo_types::Point;
use geojson::de::deserialize_geometry;
use serde::Deserialize;
use unit_conversions::length::miles::{to_feet, to_metres};
use utils::get_proximal_features;

#[derive(Deserialize)]
pub struct RoutingOptions {
    distance: f64,
    units: String,
}

#[derive(Deserialize)]
pub struct StartingGeom {
    #[serde(deserialize_with = "deserialize_geometry")]
    geometry: Point<f64>,
    node_id: u32,
}

#[debug_handler]
pub async fn get_circuit(
    State(state): State<AppState>,
    Path((schemaid, tableid)): Path<(String, String)>,
    options: Query<RoutingOptions>,
    Json(starting_geom): Json<StartingGeom>,
) -> Result<impl IntoResponse, AppError> {
    let options = options.0;
    if options.units != String::from("mins") && options.units != String::from("miles") {
        return Err(AppError(anyhow!("Missing or invalid distance units")));
    } else if options.distance <= 0.0 {
        return Err(AppError(anyhow!("Missing or invalid distance units")));
    }

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

    let mut desired_distance: f64;

    //Need to work with unit_conversions here
    //Need to pull in srid units with tables, using proj4txt column

    match &table_spec.dist_unit {
        Some(unit) => {
            desired_distance = {
                if *unit == String::from("m") {
                    to_metres(options.distance)
                } else if *unit == String::from("us-ft") {
                    to_feet(options.distance)
                } else if *unit == String::from("deg") {
                    to_metres(options.distance)
                } else {
                    options.distance
                }
            }
        }

        None => {
            return Err(AppError(anyhow!(
                "Specified table does not contain valid distance data"
            )));
        }
    };

    if options.units == String::from("mins") {
        desired_distance *= MILES_TO_MINUTES_FACTOR;
    };

    get_proximal_features(table_spec, &starting_geom, desired_distance);
    Ok(())
}
