use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRegistry {
    pub name: String,
    pub schemas: HashMap<String, Schema>,
}

impl TableRegistry {
    pub fn new(n: String) -> TableRegistry {
        TableRegistry {
            name: n,
            schemas: HashMap::new(),
        }
    }
}

impl IntoResponse for TableRegistry {
    fn into_response(self) -> axum::response::Response {
        json!(self).to_string().into_response()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub name: String,
    pub tables: HashMap<String, Table>,
}

impl Schema {
    pub fn new(n: String) -> Schema {
        Schema {
            name: n,
            tables: HashMap::new(),
        }
    }
}

//"Once DB is connected, work on loading tables in this format, and rewrite Schema"
#[derive(sqlx::FromRow, Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    #[sqlx(rename = "table")]
    pub name: String,
    #[sqlx(rename = "schema")]
    pub schema_name: String,
    pub primary_key_columns: Vec<String>,
    #[sqlx(default)]
    pub geom_column: Option<String>,
    #[sqlx(default)]
    pub geom_type: Option<String>,
    #[sqlx(default)]
    pub srid: Option<i32>,
    #[sqlx(default)]
    pub attr_columns: Option<Vec<String>>,
    #[sqlx(skip)]
    pub dist_unit: Option<String>,
    #[sqlx(skip)]
    pub use_geog: bool,
}

impl Table {
    pub fn new(
        name: String,
        schema_name: String,
        primary_key_columns: Vec<String>,
        geom_column: String,
        geom_type: String,
        srid: i32,
        attrs: Option<Vec<String>>,
        dist_unit: String,
        use_geog: bool,
    ) -> Table {
        if let Some(attr_columns) = attrs {
            Table {
                name,
                schema_name,
                primary_key_columns,
                geom_column: Some(geom_column),
                geom_type: Some(geom_type),
                srid: Some(srid),
                attr_columns: Some(attr_columns),
                dist_unit: Some(dist_unit),
                use_geog,
            }
        } else {
            Table {
                name,
                schema_name,
                primary_key_columns,
                geom_column: Some(geom_column),
                geom_type: Some(geom_type),
                srid: Some(srid),
                attr_columns: Some(Vec::new()),
                dist_unit: Some(dist_unit),
                use_geog,
            }
        }
    }
}
