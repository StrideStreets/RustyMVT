pub mod db;
pub mod geocoding;
pub mod layers;

#[macro_use]
extern crate dotenv_codegen;
extern crate dotenv;

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
