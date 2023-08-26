pub mod structs;
use anyhow::{bail, Context, Result};
use sqlx::{postgres::PgPoolOptions, query, FromRow, Pool, Postgres, Row};
use std::env::var;
use structs::{Schema, Table, TableRegistry};

pub async fn get_db_connector() -> Result<Pool<Postgres>> {
    let DB_URL =
        var("DB_CONNECTION_STRING").context("No connection string found in environment")?;
    let USER = var("DB_USER")
        .context("No DB_USER var found in environment")
        .unwrap();
    let PW = var("DB_PW")
        .context("No DB_PW var found in environment")
        .unwrap();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&DB_URL)
        .await
        .context("Failed to connect using provided connection string.")
        .unwrap();

    return Ok(pool);
}

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

//Info on making queries here: https://github.com/launchbadge/sqlx#usage
