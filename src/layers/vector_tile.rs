use crate::{db::Table, AppError};
use anyhow::{anyhow};
use axum::{http::HeaderMap, response::IntoResponse};
use reqwest::header::CONTENT_TYPE;
use sqlx::{query, Pool, Postgres, Row};

#[derive(Debug)]
pub struct Tile {
    pub z: usize,
    pub x: usize,
    pub y: usize,
}

impl Tile {
    pub fn new(x: usize, y: usize, z: usize) -> Self {
        Tile { x, y, z }
    }
}

pub struct MVTBuffer(Vec<u8>);

impl IntoResponse for MVTBuffer {
    fn into_response(self) -> axum::response::Response {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            "application/vnd.mapbox-vector-tile".parse().unwrap(),
        );
        (headers, self.0).into_response()
    }
}


const WORLD_MERC_MAX: f64 = 20037508.3427892;
const WORLD_MERC_MIN: f64 = WORLD_MERC_MAX * -1_f64;
const INCOMING_SRID: usize = 3857;
const EXTENT: f32 = 4096.0;
const BUFFER: f32 = 64.0;

/// Creates an envelope statement for a given `Tile` object and an optional margin value.
///
/// # Arguments
///
/// * `t` - A reference to a `Tile` object.
/// * `m` - An optional `f32` value representing the margin.
///
/// # Example
///
/// ```
/// let tile = Tile::new(1, 2, 3);
/// let margin = Some(0.5);
/// let envelope_statement = make_envelope_statement(&tile, margin);
/// println!("{}", envelope_statement);
/// ```
/// Expected output:
/// "ST_TileEnvelope(3, 1, 2, -20037508.3427892, -20037508.3427892, 20037508.3427892, 20037508.3427892, 3857, 0.5)"
///
/// # Returns
///
/// A formatted string representing the envelope statement for the given `Tile` object and margin value.
fn make_envelope_statement(t: &Tile, m: Option<f32>) -> String {
    let margin_text = if let Some(margin) = m {
        format!(", {}", margin.to_string())
    } else {
        "".to_string()
    };

    format!(
        "ST_TileEnvelope({}, {}, {}, ST_MakeEnvelope({}, {}, {}, {}, {}){})",
        t.z,
        t.x,
        t.y,
        WORLD_MERC_MIN,
        WORLD_MERC_MIN,
        WORLD_MERC_MAX,
        WORLD_MERC_MAX,
        INCOMING_SRID,
        margin_text
    )
}
/// Generates a SQL query string for retrieving map tile data from a database table based on a given `Tile` object and table information.
    ///
    /// # Arguments
    ///
    /// * `t` - A reference to a `Tile` object representing the map tile coordinates.
    /// * `tab` - A reference to a `Table` object representing the database table information.
    ///
    /// # Returns
    ///
    /// A `Result` object containing the SQL query string if successful, or a `TileDataQueryError` if there is an invalid tile coordinate or no geometry column found in the table.
    ///
    /// # Example
    ///
    /// ```
    /// use crate::{Tile, Table, TileDataQueryError};
    ///
    /// let tile = Tile::new(1, 2, 3);
    /// let table = Table { ... };
    /// let query = make_tile_data_query(&tile, &table)?;
    /// println!("{}", query);
    /// # Ok::<(), TileDataQueryError>(())
    /// ```
pub fn make_tile_data_query(t: &Tile, tab: &Table) -> Result<String, AppError> {

    let tile_size = 2_usize.pow(t.z as u32);
    if (t.x >= tile_size) | (t.y >= tile_size) {
        return Err(AppError(anyhow!("Invalid tile coordinates")));
    };

    if let Some(geom_col) = &tab.geom_column {
        let envelope = make_envelope_statement(t, None);

        let envelope_with_margin = make_envelope_statement(t, Some(BUFFER / EXTENT));

        let mut id_columns = tab.primary_key_columns.clone();
        let attr_columns = tab.attr_columns.clone().unwrap_or_default();
        id_columns.extend(attr_columns);

        Ok(format!(
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
                  mvtgeom;",
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
        ))
    } else {
        return Err(AppError(anyhow!("No geometry column found in table. Unable to retrieve data.")));
    }
}

pub async fn get_mvt(
    tile: &Tile,
    table: &Table,
    conn: Pool<Postgres>,
) -> Result<MVTBuffer, AppError> {
    if let Ok(mvt_query) = make_tile_data_query(tile, table) {
        match query(&mvt_query).fetch_all(&conn).await {
            Ok(mvt_result) => {
                let mvt_bytes: Vec<u8> = mvt_result[0].get(0);
                Ok(MVTBuffer(mvt_bytes))
            }
            Err(e) => {
                println!("{:?}", e);
                Err(AppError(anyhow!(format!(
                    "Failed to locate specified table. Received error {}",
                    e
                ))))
            }
        }
    } else {
        Err(AppError(anyhow!(
            "Failed to assemble MVT query from provided parameters"
        )))
    }
}

#[cfg(test)]

mod tests {

    // Should return a formatted string representing the envelope statement for a given Tile object and margin value
    #[test]
    fn should_return_formatted_string_with_envelope_statement() {
        let tile = Tile::new(1, 2, 3);
        let margin = Some(0.5);
        let envelope_statement = make_envelope_statement(&tile, margin);
        assert_eq!(
            envelope_statement,
            "ST_TileEnvelope(3, 1, 2, -20037508.3427892, -20037508.3427892, 20037508.3427892, 20037508.3427892, 3857, 0.5)"
        );
    }
    
        // Should return a formatted string with the correct ST_TileEnvelope function call
    #[test]
    fn should_return_formatted_string_with_correct_ST_TileEnvelope_function_call() {
        let tile = Tile::new(1, 2, 3);
        let margin = Some(0.5);
        let envelope_statement = make_envelope_statement(&tile, margin);
        assert!(envelope_statement.contains("ST_TileEnvelope"));
    }
    
        // Should return a formatted string with the correct ST_MakeEnvelope function call
    #[test]
    fn should_return_formatted_string_with_correct_ST_MakeEnvelope_function_call() {
        let tile = Tile::new(1, 2, 3);
        let margin = Some(0.5);
        let envelope_statement = make_envelope_statement(&tile, margin);
        assert!(envelope_statement.contains("ST_MakeEnvelope"));
    }
    
        // Should return a formatted string with no margin value when None is provided
    #[test]
    fn should_return_formatted_string_with_no_margin_value_when_None_is_provided() {
        let tile = Tile::new(1, 2, 3);
        let margin = None;
        let envelope_statement = make_envelope_statement(&tile, margin);
        assert_eq!(
            envelope_statement,
            "ST_TileEnvelope(3, 1, 2, -20037508.3427892, -20037508.3427892, 20037508.3427892, 20037508.3427892, 3857)"
        );
    }
    
        // Should return a formatted string with margin value of 0 when Some(0) is provided
    #[test]
    fn should_return_formatted_string_with_margin_value_of_0_when_Some_0_is_provided() {
        let tile = Tile::new(1, 2, 3);
        let margin = Some(0.0);
        let envelope_statement = make_envelope_statement(&tile, margin);
        assert_eq!(
            envelope_statement,
            "ST_TileEnvelope(3, 1, 2, -20037508.3427892, -20037508.3427892, 20037508.3427892, 20037508.3427892, 3857, 0.0)"
        );
    }
    
        // Should return a formatted string with margin value of 0.5 when Some(0.5) is provided
    #[test]
    fn should_return_formatted_string_with_margin_value_of_0_5_when_Some_0_5_is_provided() {
        let tile = Tile::new(1, 2, 3);
        let margin = Some(0.5);
        let envelope_statement = make_envelope_statement(&tile, margin);
        assert_eq!(
            envelope_statement,
            "ST_TileEnvelope(3, 1, 2, -20037508.3427892, -20037508.3427892, 20037508.3427892, 20037508.3427892, 3857, 0.5)"
        );
    }
}