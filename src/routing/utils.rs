use super::StartingGeom;
use crate::db::Table;
use crate::AppError;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

#[derive(sqlx::FromRow, Deserialize, Serialize)]
pub struct GeoJsonResult {
    #[sqlx(default)]
    geoms: Value,
}

/**
Retrieves a list of topological edge representations from a PostgreSQL database based distance to a given starting geometry.

# Example Usage
```rust
let pool = PgPool::new(...);
let table = Table::new(...);
let starting_geom = StartingGeom::new(...);
let desired_distance = 100.0;

let result = get_proximal_features(&pool, &table, &starting_geom, desired_distance).await;
match result {
    Ok(features) => {
        for feature in features {
            println!("{:?}", feature);
        }
    }
    Err(error) => {
        println!("Error: {}", error);
    }
}
```

# Arguments
* `pool` - A PostgreSQL connection pool.
* `table` - A struct representing the database table.
* `starting_geom` - A struct representing the starting geometry.
* `desired_distance` - The desired distance for proximity search.

# Returns
A `Result` containing either a vector of `TopoEdgeRepresentation` structs if the query is successful, or an `AppError` if there is an error.
*/
pub async fn get_proximal_features(
    pool: &PgPool,
    table: &Table,
    starting_geom: &StartingGeom,
    desired_distance: f64,
) -> Result<Vec<TopoEdgeRepresentation>, AppError> {
    let geom_col = match table.geom_column.as_ref() {
        Some(geom_col) => geom_col,
        None => {
            return Err(AppError(anyhow!(
                "Table does not contain valid geometry data"
            )))
        }
    };
    let srid = match table.srid.as_ref() {
        Some(srid) => srid,
        None => {
            return Err(AppError(anyhow!(
                "Table does not contain valid geometry data"
            )))
        }
    };

    let starting_coords = (starting_geom.geometry.x(), starting_geom.geometry.y());
    let use_geog = if table.use_geog { "::geography" } else { "" };

    let geom_restrictor = format!(
        "WHERE ST_DWithin(t.{}{},
        ST_Transform(ST_SetSRID(ST_MakePoint({},{}), 3857), {}){}, {})",
        geom_col, use_geog, starting_coords.0, starting_coords.1, srid, use_geog, desired_distance
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
        pk_string, attrs_string, geom_col, use_geog, table.schema_name, table.name, geom_restrictor
    );

    println!("{}", &proximal_features_query);

    query_as::<Postgres, TopoEdgeRepresentation>(&proximal_features_query)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError(anyhow!(e)))
}

/*
Create a mapping between pairs of start and end nodes in a list of `TopoEdgeRepresentation` structs and their corresponding edge IDs.

# Arguments

* `edges` - A slice of `TopoEdgeRepresentation` structs representing the edges.

# Example

```rust
let edges = vec![
    TopoEdgeRepresentation { edge_id: 1, start_node: 2, end_node: 3, weight: 0.0 },
    TopoEdgeRepresentation { edge_id: 2, start_node: 3, end_node: 4, weight: 0.0 },
    TopoEdgeRepresentation { edge_id: 3, start_node: 4, end_node: 5, weight: 0.0 },
];

let mapper = get_edge_to_vertex_pair_mapper(&edges);

// The expected output is a HashMap with the following mappings:
// (2, 3) -> 1
// (3, 4) -> 2
// (4, 5) -> 3
```
*/
pub fn get_edge_to_vertex_pair_mapper(
    edges: &[TopoEdgeRepresentation],
) -> HashMap<(u32, u32), u32> {
    let mut pair_to_edge_mapper = HashMap::with_capacity(edges.len() * 2);
    for e in edges {
        pair_to_edge_mapper
            .entry((e.start_node, e.end_node))
            .or_insert(e.edge_id);
        pair_to_edge_mapper
            .entry((e.end_node, e.start_node))
            .or_insert(e.edge_id);
    }

    pair_to_edge_mapper
}

/**
Tries to convert a slice of `TopoEdgeRepresentation` structs into a JSON string.

# Example

```
let edges = vec![
    TopoEdgeRepresentation { edge_id: 1, start_node: 2, end_node: 3, weight: 0.0 },
    TopoEdgeRepresentation { edge_id: 2, start_node: 3, end_node: 4, weight: 0.0 },
    TopoEdgeRepresentation { edge_id: 3, start_node: 4, end_node: 5, weight: 0.0 },
];

let result = try_convert_to_edge_json(&edges);

// The expected output is a Result containing a JSON string representation of the edges.
```

# Arguments

* `rows` - A slice of `TopoEdgeRepresentation` structs to be converted to JSON.

# Returns

A `Result` containing either a JSON string representation of the edges if serialization is successful, or an `AppError` if serialization fails.
*/
pub fn try_convert_to_edge_json(rows: &[TopoEdgeRepresentation]) -> Result<String, AppError> {
    serde_json::to_string(rows)
        .map_err(|e| AppError(anyhow!("Failed to convert rows to JSON: {}", e)))
}

/**
Processes a routing result represented as a list of node IDs and converts it into a list of edge IDs using a mapping between node pairs and edge IDs.

# Example

```
use std::collections::HashMap;

let results = vec![1, 2, 3, 4];
let nodes_to_edge_mapper: HashMap<(u32, u32), u32> = [
    ((1, 2), 10),
    ((2, 3), 20),
    ((3, 4), 30),
].iter().cloned().collect();

let edges = process_routing_result_as_edge_list(&results, &nodes_to_edge_mapper);

// The expected output is a list of edge IDs: [Some("10"), Some("20"), Some("30")]
```

# Arguments

* `results` - A list of node IDs representing the routing result.
* `nodes_to_edge_mapper` - A mapping between pairs of start and end nodes and their corresponding edge IDs.

# Returns

A list of edge IDs corresponding to the routing result.
*/
pub fn process_routing_result_as_edge_list(
    results: &[u32],
    nodes_to_edge_mapper: &HashMap<(u32, u32), u32>,
) -> Vec<Option<String>> {
    let edges: Vec<Option<String>> = results
        .iter()
        .zip(results.iter().skip(1))
        .map(|(u, v)| {
            nodes_to_edge_mapper
                .get(&(*u, *v))
                .and_then(|eid| Some(eid.to_string()))
        })
        .collect();

    edges
}

/**
Retrieves the geometries associated with a given path of edge IDs from a PostgreSQL database.

# Arguments

* `path` - A list of edge IDs representing the path.
* `table` - A struct representing the database table.
* `pool` - A PostgreSQL connection pool.

# Returns

A `Result` containing either a JSON string representation of the geometries if the query is successful, or an `AppError` if there is an error.
*/
pub async fn get_path_geometries(
    path: Vec<Option<String>>,
    table: &Table,
    pool: &PgPool,
) -> Result<String, AppError> {
    if let Some(geom_col) = &table.geom_column {
        let primary_key_column = match table.primary_key_columns.get(0) {
            Some(column) => column,
            None => {
                return Err(AppError(anyhow!(
                    "Table does not contain valid primary key columns"
                )))
            }
        };

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
            primary_key_column,
            format!(
                "{}",
                path.iter()
                    .filter_map(|item| item.clone())
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
