mod vector_tile;
use anyhow::anyhow;
use axum::extract::{Path, State};
use vector_tile::{get_mvt, MVTBuffer, Tile};

use axum_macros::debug_handler;

use crate::{AppError, AppState};

#[debug_handler]
/**
Handles a GET request to retrieve a specific layer of a vector tile.

# Arguments

* `State(state)`: The state of the application, which contains the table registry and database pool.
* `Path((schemaid, tableid, z, x, y_ext))`: The path parameters extracted from the request URL, including the schema ID, table ID, zoom level, tile coordinates, and file extension.

# Returns

Returns a result that either contains the `MVTBuffer` containing the vector tile data or an `AppError` if there was an error during the process.

# Example

```rust
// Assuming the following inputs:
let state = AppState { ... };
let schemaid = "schema1";
let tableid = "table1";
let z = 10;
let x = 123;
let y_ext = "456.mvt";

// Calling the function:
let result = get_layer(State(state), Path((schemaid.to_string(), tableid.to_string(), z, x, y_ext.to_string()))).await;

// Expected output:
// If the specified schema and table exist, and the file extension is "mvt", the function will return the MVTBuffer containing the vector tile data. Otherwise, it will return an AppError.
```
*/
pub async fn get_layer(
    State(state): State<AppState>,
    Path((schemaid, tableid, z, x, y_ext)): Path<(String, String, usize, usize, String)>,
) -> Result<MVTBuffer, AppError> {
    let (y, ext) = match y_ext.split('.').collect::<Vec<&str>>().as_slice() {
        [y_str, ext] => {
            match y_str.parse::<usize>() {
                Ok(y) => (y, ext.to_string()),
                Err(_) => return Err(AppError(anyhow!("Failed to parse y_str as usize"))),
            }
        }
        _ => return Err(AppError(anyhow!("Invalid y_ext format: {}", y_ext))),
    };

    let table_spec = if let Some(schema) = state.table_registry.schemas.get(&schemaid) {
        if let Some(table) = schema.tables.get(&tableid) {
            table
        } else {
            return Err(AppError(anyhow!("Failed to locate specified table")));
        }
    } else {
        return Err(AppError(anyhow!("Failed to locate specified schema")));
    };

    let this_tile = Tile::new(x, y, z);

    match ext.as_str() {
        "mvt" => get_mvt(&this_tile, table_spec, state.db_pool).await,
        _ => Err(AppError(anyhow!("Specified file extension not supported"))),
    }
}
