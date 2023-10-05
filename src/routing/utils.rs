use crate::db::Table;

use super::StartingGeom;
pub fn get_proximal_features(table: &Table, starting_geom: &StartingGeom, desired_distance: f64) {
    if let (Some(geom_col), Some(srid)) = (&table.geom_column, &table.srid) {
        let starting_coords = (starting_geom.geometry.x(), starting_geom.geometry.y());

        let geom_restrictor = format!(
            "WHERE ST_DWithin(t.{}::geography,
            ST_Transform(ST_SetSRID(ST_MakePoint({},{}), 3857), {})::geography, {})",
            geom_col, starting_coords.0, starting_coords.1, srid, desired_distance
        );
    }
}
