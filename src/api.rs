use crate::db::structs::Schema;

use super::db::structs::{Table, TableRegistry};
use super::AppState;
use anyhow::{anyhow, bail, Context, Result};
use axum::extract::{Path, State};
use sqlx::{query, FromRow, Pool, Postgres, Row};

struct Tile {
    z: u32,
    x: u32,
    y: u32,
}

const WORLD_MERC_MAX: f64 = 20037508.3427892;
const WORLD_MERC_MIN: f64 = WORLD_MERC_MAX * -1_f64;
const INCOMING_SRID: usize = 3857;
const EXTENT: f32 = 4096.0;
const BUFFER: f32 = 64.0;

fn get_table(r: &TableRegistry, schema: String, table: String) -> Result<&Table> {
    return r
        .schemas
        .get(&schema)
        .and_then(|s: &Schema| s.tables.get(&table))
        .ok_or(anyhow!("Specified table not found in schema"));
}

fn make_envelope_statement(t: &Tile, m: Option<f32>) -> String {
    let mut margin_text: String = "".to_string();
    if let Some(margin) = m {
        margin_text = format!(", {}", margin);
    };

    return format!(
        "ST_TileEnvelope({}, {}, {}, ST_MakeEnvelope({}, {}, {}, {}, {}{}))",
        t.z,
        t.x,
        t.y,
        WORLD_MERC_MIN,
        WORLD_MERC_MIN,
        WORLD_MERC_MAX,
        WORLD_MERC_MAX,
        INCOMING_SRID,
        margin_text
    );
}

fn make_tile_data_query(t: &Tile, tab: &Table) -> Result<String> {
    //It occurs to me that we need some way to select what attributes should be served up with the tile
    let tile_size = 2_u32.pow(t.z);
    if (t.x >= tile_size) | (t.y >= tile_size) {
        bail!("Invalid tile coordinates");
    };

    if let Some(geom_col) = &tab.geom_column {
        let envelope = make_envelope_statement(t, None);

        let envelope_with_margin = make_envelope_statement(t, Some(BUFFER / EXTENT));

        let mut id_columns = tab.primary_key_columns.clone();
        let attr_columns = tab.attr_columns.clone().unwrap_or_default();
        id_columns.extend(attr_columns.into_iter());

        let mvt_query = format!(
            "with mvtgeom as (
                select
                    ST_AsMVTGeom(
                            ST_Transform(t.{},
                    {}),
                    {},
                    {},
                    {}
                        ) as geom,
                    {}
                from
                    {}.{} t
                where
                    ST_Transform(t.{},
                    {}) && {})
                    select
                    ST_AsMVT(mvtgeom.*)
                from
                    mvtgeom
                    );",
            geom_col,
            INCOMING_SRID,
            envelope,
            EXTENT,
            BUFFER,
            id_columns.join(", "),
            tab.schema_name,
            tab.name,
            geom_col,
            INCOMING_SRID,
            envelope_with_margin
        );
    } else {
        bail!("No geometry column found in table. Unable to retrieve data.")
    }
    return Ok("String".to_string());
}

pub async fn serve_tile(
    Path((schema, table, x, y, z)): Path<(String, String, u32, u32, u32)>,
    State(state): State<AppState>,
) -> String {
    let pool = state.db_pool;

    let table_registry = state.table_registry;

    let mvt_query = get_table(&table_registry, schema, table)
        .context("Unable to locate requested table")
        .and_then(|tab| make_tile_data_query(&Tile { x, y, z }, tab))
        .context("Encountered error while assembling query text")
        .unwrap();

    let query_results = query(&mvt_query)
        .fetch_all(&pool)
        .await
        .context("Encountered error while processing MVT query")
        .unwrap();

    println!("{:?}", table_registry);

    fn execute_query() {
        todo!()
    }

    fn handle_results() {
        todo!()
    }

    return "Placeholder String!".to_owned();
}
