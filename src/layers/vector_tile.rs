use crate::{db::Table, AppError};
use anyhow::{anyhow, bail, Error};
use axum::{http::HeaderMap, response::IntoResponse};
use reqwest::header::{ACCESS_CONTROL_ALLOW_ORIGIN, CONTENT_TYPE};
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
        headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
        (headers, self.0).into_response()
    }
}

const WORLD_MERC_MAX: f64 = 20037508.3427892;
const WORLD_MERC_MIN: f64 = WORLD_MERC_MAX * -1_f64;
const INCOMING_SRID: usize = 3857;
const EXTENT: f32 = 4096.0;
const BUFFER: f32 = 64.0;

fn make_envelope_statement(t: &Tile, m: Option<f32>) -> String {
    let mut margin_text: String = "".to_string();
    if let Some(margin) = m {
        margin_text = format!(", {}", margin);
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

pub fn make_tile_data_query(t: &Tile, tab: &Table) -> Result<String, Error> {
    //It occurs to me that we need some way to select what attributes should be served up with the tile
    let tile_size = 2_usize.pow(t.z as u32);
    if (t.x >= tile_size) | (t.y >= tile_size) {
        bail!("Invalid tile coordinates");
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
        bail!("No geometry column found in table. Unable to retrieve data.")
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
