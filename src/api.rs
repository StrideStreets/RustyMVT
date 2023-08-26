use super::AppState;
use anyhow::{anyhow, bail, Context, Result};
use axum::extract::{Path, State};
use sqlx::{query, FromRow, Pool, Postgres, Row};
use std::{collections::HashMap, fmt::Debug};

#[derive(Debug)]
pub struct TableRegistry {
    name: String,
    schemas: HashMap<String, Schema>,
}

impl TableRegistry {
    fn new(n: String) -> TableRegistry {
        return TableRegistry {
            name: n,
            schemas: HashMap::new(),
        };
    }
}

#[derive(Debug)]
struct Schema {
    name: String,
    tables: HashMap<String, Table>,
}

impl Schema {
    fn new(n: String) -> Schema {
        return Schema {
            name: n,
            tables: HashMap::new(),
        };
    }
}

//"Once DB is connected, work on loading tables in this format, and rewrite Schema"
#[derive(sqlx::FromRow, Debug)]
struct Table {
    #[sqlx(rename = "table")]
    name: String,
    primary_key_columns: Vec<String>,
    #[sqlx(default)]
    geom_column: Option<String>,
    #[sqlx(default)]
    geom_type: Option<String>,
    #[sqlx(default)]
    srid: Option<i32>,
    #[sqlx(default)]
    attr_columns: Option<Vec<String>>,
}

impl Table {
    fn new(
        name: String,
        primary_key_columns: Vec<String>,
        geom_column: String,
        geom_type: String,
        srid: i32,
        attrs: Option<Vec<String>>,
    ) -> Table {
        if let Some(attr_columns) = attrs {
            return Table {
                name,
                primary_key_columns,
                geom_column: Some(geom_column),
                geom_type: Some(geom_type),
                srid: Some(srid),
                attr_columns: Some(attr_columns),
            };
        } else {
            return Table {
                name,
                primary_key_columns,
                geom_column: Some(geom_column),
                geom_type: Some(geom_type),
                srid: Some(srid),
                attr_columns: Some(Vec::new()),
            };
        }
    }
}

struct Tile {
    z: u32,
    x: u32,
    y: u32,
}

const WORLD_MERC_MAX: f64 = 20037508.3427892;
const WORLD_MERC_MIN: f64 = WORLD_MERC_MAX * -1_f64;
const INCOMING_SRID: usize = 3857;

pub async fn load_table_registry(p: &Pool<Postgres>, db: String) -> Result<TableRegistry> {
    //Will autopopulate a table registry for a given database, in the mold of the placeholder defined below

    let mut schema_and_table_info = query("select
    tabs.*,
    gc.f_geometry_column as geom_column, gc.srid as srid, gc.type as geom_type, gc.coord_dimension as geom_coord_dimension
    from
    (
        select
        pks.schema_name as schema,
        pks.table_name as table,
        array_agg(pks.pk)::TEXT[] as primary_key_columns
        from
        (
            select
            tab.table_schema as schema_name,
            tab.table_name as table_name,
            tco.column_name as pk
            from
            information_schema.table_constraints tab
            left join information_schema.key_column_usage tco on
            tab.table_schema = tco.table_schema
            and tab.table_name = tco.table_name
            and tab.constraint_name = tco.constraint_name
            where
            tab.constraint_type = 'PRIMARY KEY'
            and tab.table_schema <> 'pg_catalog'
            and tco.ordinal_position is not null
            order by
            tab.table_schema,
            tab.table_name,
            tco.position_in_unique_constraint
            
        ) pks
        group by
        pks.schema_name,
        pks.table_name) tabs
        left join geometry_columns gc 
        on
        tabs.schema = gc.f_table_schema
        and tabs.table = gc.f_table_name").fetch_all(p).await.context("Encountered error while querying database for schemata").unwrap().into_iter();

    let mut registry = TableRegistry::new(db);

    while let Some(row) = schema_and_table_info.next() {
        let schema_name = &row.try_get::<String, &str>("schema");
        let table_name = &row.try_get::<String, &str>("table");
        let geo_column = &row.try_get::<String, &str>("geometry_column");

        if let (Ok(schema), Ok(table)) = (schema_name, table_name) {
            match registry.schemas.get_mut(schema) {
                Some(schema) => {
                    schema.tables.insert(
                        table.to_string(),
                        Table::from_row(&row)
                            .context("Encountered error while converting row fields to Table")
                            .unwrap(),
                    );
                }
                None => {
                    registry
                        .schemas
                        .insert(schema.to_string(), Schema::new(schema.to_string()));

                    let local_schema = registry.schemas.get_mut(schema).unwrap();

                    local_schema.tables.insert(
                        table.to_string(),
                        Table::from_row(&row)
                            .context("Encoutered error while converting row fields to Table")
                            .unwrap(),
                    );
                }
            }
        } else {
            bail!("Schema name not found in row")
        }
    }

    return Ok(registry);
}

fn check_registry_for_table(r: TableRegistry, schema: String, table: String) -> Result<()> {
    let res = match r.schemas.get(&schema) {
        Some(s) => s.tables.contains_key(&table),
        None => false,
    };

    match res {
        true => return Ok(()),
        false => return Err(anyhow!("Table not found.")),
    }
}

fn parse_coordinates_to_envelope_statement(t: Tile) -> Result<String> {
    let tile_size = 2_u32.pow(t.z);
    if (t.x >= tile_size) | (t.y >= tile_size) {
        bail!("Invalid tile coordinates");
    };

    return Ok(format!(
        "ST_TileEnvelope({}, {}, {}, ST_MakeEnvelope({}, {}, {}, {}, {}))",
        t.z,
        t.x,
        t.y,
        WORLD_MERC_MIN,
        WORLD_MERC_MIN,
        WORLD_MERC_MAX,
        WORLD_MERC_MAX,
        INCOMING_SRID
    ));
}

fn make_full_query(envelope: String) {
    //It occurs to me that we need some way to select what attributes should be served up with the tile
}

pub async fn serve_tile(
    Path((schema, table, x, y, z)): Path<(String, String, u32, u32, u32)>,
    State(state): State<AppState>,
) -> String {
    let pool = state.db_pool;
    let table_registry_placeholder = load_table_registry(&pool, "default".to_string())
        .await
        .context("Encountered error while loading table registry")
        .unwrap();

    println!("{:?}", table_registry_placeholder);

    fn execute_query() {
        todo!()
    }

    fn handle_results() {
        todo!()
    }

    return "Placeholder String!".to_owned();
}
