mod utils;
use std::collections::VecDeque;

use crate::{AppError, AppState, MILES_TO_MINUTES_FACTOR};
use anyhow::anyhow;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use axum_macros::debug_handler;
use geo_types::Point;
use geojson::de::{deserialize_geometry, deserialize_single_feature};
use serde::Deserialize;
use speedicycle::make_route_from_edges_json;
use unit_conversions::length::miles::{to_feet, to_metres};
use utils::{
    get_edge_to_vertex_pair_mapper, get_path_geometries, get_proximal_features,
    process_routing_result_as_edge_list,
};

use self::utils::try_convert_to_edge_json;
use tokio::task::spawn;

#[derive(Deserialize)]
pub struct RoutingOptions {
    dist: f64,
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
    Json(raw_starting_geom): Json<String>, //Json(starting_geom): Json<StartingGeom>,
) -> Result<Json<VecDeque<String>>, AppError> {
    let options = options.0;
    if options.units != String::from("mins") && options.units != String::from("miles") {
        return Err(AppError(anyhow!("Missing or invalid distance units")));
    } else if options.dist <= 0.0 {
        return Err(AppError(anyhow!("Missing or invalid distance units")));
    }

    println!("{}", &raw_starting_geom);
    let starting_geom: StartingGeom;

    match deserialize_single_feature::<StartingGeom>(raw_starting_geom.as_bytes()) {
        Ok(geom) => {
            starting_geom = geom;
        }
        Err(e) => {
            return Err(AppError(anyhow!(e)));
        }
    };

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
    println!("{:?}", &table_spec);

    match &table_spec.dist_unit {
        Some(unit) => {
            desired_distance = {
                if *unit == String::from("m") {
                    to_metres(options.dist)
                } else if *unit == String::from("us-ft") {
                    to_feet(options.dist)
                } else if *unit == String::from("deg") {
                    to_metres(options.dist)
                } else {
                    options.dist
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

    let proximal_features =
        get_proximal_features(&state.db_pool, table_spec, &starting_geom, desired_distance).await;

    match proximal_features {
        Ok(rows) => {
            let edge_to_vertex_mapper = get_edge_to_vertex_pair_mapper(&rows);

            match try_convert_to_edge_json(&rows) {
                Ok(json_str) => {
                    let routing_task = spawn(async move {
                        make_route_from_edges_json::<u32, f64, u32>(
                            json_str,
                            starting_geom.node_id,
                            desired_distance,
                        )
                    })
                    .await?;
                    match routing_task.map_err(|e| AppError(anyhow!(e))) {
                        Ok(results) => {
                            let mut valid_paths = VecDeque::new();
                            let upper_edges = process_routing_result_as_edge_list(
                                results.upper,
                                &edge_to_vertex_mapper,
                            );
                            let lower_edges = process_routing_result_as_edge_list(
                                results.lower,
                                &edge_to_vertex_mapper,
                            );
                            if let Ok(path) =
                                get_path_geometries(upper_edges, table_spec, &state.db_pool).await
                            {
                                valid_paths.push_back(path);
                            }
                            if let Ok(path) =
                                get_path_geometries(lower_edges, table_spec, &state.db_pool).await
                            {
                                valid_paths.push_back(path);
                            }
                            return Ok(Json(valid_paths));
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
        Err(e) => {
            return Err(e);
        }
    }
}
