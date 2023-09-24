use crate::{db::structs::Table, structs::Tile};
use anyhow::bail;
use anyhow::Error;
use axum::response::IntoResponse;
use axum::response::Response;
use reqwest::StatusCode;

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

    return format!(
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
    );
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
        id_columns.extend(attr_columns.into_iter());

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
