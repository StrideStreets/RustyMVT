mod structs;
use crate::get_srid_unit;
use anyhow::{anyhow, Context, Result};
use sqlx::{
    postgres::{PgPool, PgPoolOptions},
    query, FromRow, Pool, Postgres, Row,
};
use std::env::var;
pub use structs::{Schema, Table, TableRegistry};

/**
Retrieves a database connection pool for a PostgreSQL database.

# Example

```rust
# use anyhow::Result;
# use sqlx::{Pool, Postgres};
#
# pub async fn var(_: &str) -> Result<String> { Ok("".to_string()) }
#
# pub async fn get_db_connector() -> Result<Pool<Postgres>> {
let pool = get_db_connector().await?;
#     Ok(pool)
# }
```

# Errors

- If the environment variable `DB_CONNECTION_STRING` is not found or empty, returns an `anyhow::Error`.
- If the environment variable `DB_MAX_CONNECTIONS` is not found or cannot be parsed as a `u32`, returns an `anyhow::Error`.
- If connecting to the database using the provided connection string fails, returns an `anyhow::Error`.

# Returns

A `Result` containing the PostgreSQL connection pool. If successful, the connection pool is returned. If an error occurs, an `anyhow::Error` is returned.
*/
pub async fn get_db_connector() -> Result<Pool<Postgres>> {
    let db_url =
        var("DB_CONNECTION_STRING").context("No connection string found in environment")?;

    let max_connections = var("DB_MAX_CONNECTIONS")
        .context("No DB_MAX_CONNECTIONS var found in environment")
        .unwrap_or_else(|_| "5".to_string())
        .parse::<u32>()
        .context("Failed to parse DB_MAX_CONNECTIONS as u32")?;

    match PgPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(1)
        .connect(&db_url)
        .await
    {
        Ok(pool) => return Ok(pool),

        Err(e) => return Err(anyhow!("Failed to connect using provided string: {}", e)),
    }
}

/**
Retrieves information about tables and schemas from a PostgreSQL database.

# Arguments

* `p` - A reference to a `PgPool` object representing the connection pool to the PostgreSQL database.
* `db` - A `String` containing the name of the database.

# Returns

A `Result` containing the populated `TableRegistry` object. If the function is successful, the `TableRegistry` object is returned. If an error occurs, an `anyhow::Error` is returned.

# Example

```rust
# use anyhow::Result;
# use sqlx::{Pool, Postgres};
#
# pub async fn var(_: &str) -> Result<String> { Ok("".to_string()) }
#
# pub async fn get_db_connector() -> Result<Pool<Postgres>> {
#     let pool = get_db_connector().await?;
#     Ok(pool)
# }
#
# pub async fn get_srid_unit(_: i32) -> Option<String> { Some("".to_string()) }
#
# mod structs {
#     pub struct Schema {
#         pub tables: std::collections::HashMap<String, Table>,
#     }
#
#     pub struct Table {
#         pub srid: Option<i32>,
#         pub dist_unit: Option<String>,
#         pub use_geog: bool,
#     }
#
#     pub struct TableRegistry {
#         pub schemas: std::collections::HashMap<String, Schema>,
#     }
# }
#
# async fn query(_: &str) -> Result<Vec<sqlx::postgres::PgRow>> { Ok(Vec::new()) }
#
# async fn fetch_all(_: &sqlx::postgres::PgPool) -> Result<Vec<sqlx::postgres::PgRow>> { Ok(Vec::new()) }
#
# async fn get_srid_unit(_: i32) -> Option<String> { Some("".to_string()) }
#
# async fn get_db_connector() -> Result<sqlx::postgres::PgPool> { Ok(sqlx::postgres::PgPool::new("")) }
#
# async fn load_table_registry(p: &sqlx::postgres::PgPool, db: String) -> Result<structs::TableRegistry> {
let pool = get_db_connector().await?;
let registry = load_table_registry(&pool, "my_database".to_string()).await?;
#     Ok(registry)
# }
```
*/
pub async fn load_table_registry(p: &PgPool, db: String) -> Result<TableRegistry> {
    let schema_and_table_info = query("select
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
        and tabs.table = gc.f_table_name")
        .fetch_all(p)
        .await
        .context("Encountered error while querying database for schemata")?
        .into_iter();

    let mut registry = TableRegistry::new(db);

    for row in schema_and_table_info {
        let schema_name = row
            .try_get::<String, &str>("schema")
            .context("Schema name not found in row")?;
        let table_name = row
            .try_get::<String, &str>("table")
            .context("Table name not found in row")?;

        let _geo_column = row.try_get::<String, &str>("geom_column").ok();

        let schema_name = schema_name.to_owned();
        let table_name = table_name.to_owned();

        let mut this_table = Table::from_row(&row)
            .context("Encountered error while converting row fields to Table")?;

        if let Some(srid) = this_table.srid {
            this_table.dist_unit = get_srid_unit(srid).map(|unit| unit.to_owned());
            if let Some(unit) = &this_table.dist_unit {
                this_table.use_geog = *unit == "deg".to_string();
            } else {
                this_table.use_geog = false;
            }
        }

        registry
            .schemas
            .entry(schema_name)
            .or_insert_with_key(|key| Schema::new(key.to_string()))
            .tables
            .insert(table_name, this_table);
    }

    Ok(registry)
}

//Info on making queries here: https://github.com/launchbadge/sqlx#usage
