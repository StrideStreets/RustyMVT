use super::StartingGeom;
use crate::db::Table;
use crate::AppError;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::{ser::to_string, Value};
use sqlx::{query_as, PgPool, Postgres};
use std::collections::HashMap;

#[derive(sqlx::FromRow, Debug, Clone, Deserialize, Serialize)]
pub struct TopoEdgeRepresentation {
    #[sqlx(try_from = "i32")]
    edge_id: u32,
    #[sqlx(try_from = "i32")]
    start_node: u32,
    #[sqlx(try_from = "i32")]
    end_node: u32,
    #[sqlx(default)]
    weight: f64,
}

// impl FromRow<'_, PgRow> for TopoEdgeRepresentation {
//     fn from_row(row: &'_ PgRow) -> Result<Self, sqlx::Error> {
//         Ok(Self {
//             edge_id: match row.try_get::<i32, &str>("edge_id") {
//                 Ok(val) => format!("{}", val),
//                 Err(e) => return Err(sqlx::Error::Decode(e.into())),
//             },

//             start_node: match row.try_get("start_node").and_then(|val: i64| {
//                 val.try_into()
//                     .map_err(|e: TryFromIntError| sqlx::Error::Decode(e.into()))
//             }) {
//                 Ok(val) => val,
//                 Err(e) => return Err(e),
//             },
//             end_node: match row.try_get("end_node").and_then(|val: i64| {
//                 val.try_into()
//                     .map_err(|e: TryFromIntError| sqlx::Error::Decode(e.into()))
//             }) {
//                 Ok(val) => val,
//                 Err(e) => return Err(e),
//             },
//             weight: match row.try_get("weight") {
//                 Ok(val) => val,
//                 Err(e) => return Err(e),
//             },
//         })
//     }
// }

#[derive(sqlx::FromRow, Deserialize, Serialize)]
pub struct GeoJsonResult {
    #[sqlx(default)]
    geoms: Value,
}

pub async fn get_proximal_features(
    pool: &PgPool,
    table: &Table,
    starting_geom: &StartingGeom,
    desired_distance: f64,
) -> Result<Vec<TopoEdgeRepresentation>, AppError> {
    if let (Some(geom_col), Some(srid)) = (&table.geom_column, &table.srid) {
        let starting_coords = (starting_geom.geometry.x(), starting_geom.geometry.y());
        let use_geog = match table.use_geog {
            true => "::geography",
            false => "",
        };

        //Eventually, use geom conditionally depending on layer unit
        let geom_restrictor = format!(
            "WHERE ST_DWithin(t.{}{},
            ST_Transform(ST_SetSRID(ST_MakePoint({},{}), 3857), {}){}, {})",
            geom_col,
            use_geog,
            starting_coords.0,
            starting_coords.1,
            srid,
            use_geog,
            desired_distance
        );

        let pk_string = format!("{}", table.primary_key_columns.join(", "));
        let attrs_string = match &table.attr_columns {
            Some(columns) => format!(", {}", columns.join(", ")),
            None => ", start_node, end_node".to_string(),
        };
        let proximal_features_query = format!(
            "select {}{}, trunc(ST_Length(t.{}{})) as weight
            from {}.{} t
            {}
            ",
            pk_string,
            attrs_string,
            geom_col,
            use_geog,
            table.schema_name,
            table.name,
            geom_restrictor
        );

        println!("{}", &proximal_features_query);

        query_as::<Postgres, TopoEdgeRepresentation>(&proximal_features_query)
            .fetch_all(pool)
            .await
            .map_err(|e| AppError(anyhow!(e)))
    } else {
        Err(AppError(anyhow!(
            "Table does not contain valid geometry data"
        )))
    }
}

pub fn get_edge_to_vertex_pair_mapper(
    edges: &Vec<TopoEdgeRepresentation>,
) -> HashMap<(u32, u32), u32> {
    let mut pair_to_edge_mapper = HashMap::new();
    edges.iter().for_each(|e| {
        pair_to_edge_mapper.insert((e.start_node, e.end_node), e.edge_id.clone());
        pair_to_edge_mapper.insert((e.end_node, e.start_node), e.edge_id.clone());
    });

    pair_to_edge_mapper
}
pub fn try_convert_to_edge_json(rows: &Vec<TopoEdgeRepresentation>) -> Result<String, AppError> {
    to_string(&rows).map_err(|e| AppError(anyhow!(e)))
}

//This method will eventually take RoutingResults<u32> once avail in public Speedicycle crate. For now, recreate struct here:

pub fn process_routing_result_as_edge_list(
    results: Vec<u32>,
    nodes_to_edge_mapper: &HashMap<(u32, u32), u32>,
) -> Vec<Option<String>> {
    let edges: Vec<Option<String>> = results
        .iter()
        .zip(results.iter().skip(1))
        .map(|(u, v)| {
            nodes_to_edge_mapper
                .get(&(*u, *v))
                .and_then(|eid| Some(format!("{}", eid)))
        })
        .collect();

    edges
}

pub async fn get_path_geometries(
    path: Vec<Option<String>>,
    table: &Table,
    pool: &PgPool,
) -> Result<String, AppError> {
    if let Some(geom_col) = &table.geom_column {
        let geojson_query = format!(
            "select st_asgeojson(c.*)::json as geoms
                from
                    (
                    select
                        ST_Collect(
                    array(
                        select
                            ST_Transform(t.{},
                            3857) as geom
                        from
                            {}.{} t
                        where
                            t.{} in ({})
                        )
                    )
                    ) c;",
            geom_col,
            table.schema_name,
            table.name,
            table.primary_key_columns[0],
            format!(
                "{}",
                path.iter()
                    .filter_map(|item| {
                        match item {
                            Some(s) => Some(s.clone()),
                            None => None,
                        }
                    })
                    .collect::<Vec<String>>()
                    .join(", ")
            )
        );

        println!("{}", &geojson_query);

        query_as::<Postgres, GeoJsonResult>(&geojson_query)
            .fetch_one(pool)
            .await
            .map(|gjs| gjs.geoms.to_string())
            .map_err(|e| AppError(anyhow!(e)))
    } else {
        Err(AppError(anyhow!("Failed to fetch geometries as GeoJSON")))
    }
}
