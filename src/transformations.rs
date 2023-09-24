use super::db::structs::{Schema, Table, TableRegistry};
use super::AppState;
use anyhow::{anyhow, Context, Result};
use axum::extract::{Path, State};

pub mod vector_tile;

fn get_table(r: &TableRegistry, schema: String, table: String) -> Result<&Table> {
    return r
        .schemas
        .get(&schema)
        .and_then(|s: &Schema| s.tables.get(&table))
        .ok_or(anyhow!("Specified table not found in schema"));
}
