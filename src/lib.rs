include!(concat!(env!("OUT_DIR"), "/codegen.rs"));
pub mod db;
pub mod geocoding;
pub mod layers;
pub mod routing;

use crate::db::TableRegistry;
use axum::response::{IntoResponse, Response};
use reqwest::StatusCode;
use sqlx::{Pool, Postgres};

pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db_pool: Pool<Postgres>,
    pub table_registry: TableRegistry,
}

pub fn get_srid_unit(srid: i32) -> Option<&'static str> {
    UNIT_BY_SRID.get(&srid).copied()
}

pub static MILES_TO_MINUTES_FACTOR: f64 = 0.05;
