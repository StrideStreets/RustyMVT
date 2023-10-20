use crate::AppError;
extern crate dotenv;
use axum::{extract::Path, response::IntoResponse, Json};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue, ACCESS_CONTROL_ALLOW_ORIGIN},
    Client,
};
use serde::{de, Deserialize, Deserializer, Serialize};
use serde_json::Value;

use anyhow::anyhow;
use axum_macros::debug_handler;
use lazy_static::lazy_static;
use urlencoding::encode;

lazy_static! {
    static ref CLIENT: Client = Client::new();
}

//Error handling here is awful. Clean up after getting the shell working.

fn deserialize_to_f64<'de, D: Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
    Ok(match Value::deserialize(d)? {
        Value::String(s) => s.parse().map_err(de::Error::custom)?,
        Value::Number(num) => num.as_f64().ok_or(de::Error::custom(""))?,
        _ => return Err(de::Error::custom("")),
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GeocoderResult {
    #[serde(deserialize_with = "deserialize_to_f64")]
    lat: f64,
    #[serde(deserialize_with = "deserialize_to_f64")]
    lon: f64,
}

impl IntoResponse for GeocoderResult {
    fn into_response(self) -> axum::response::Response {
        let mut headers = HeaderMap::new();
        headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
        (headers, Json(self)).into_response()
    }
}

/**
Makes an API call to a geocoding service to retrieve latitude and longitude coordinates based on a given query string.

# Arguments

* `query_string` - The query string used to search for a location in the geocoding service.
* `client` - A reference to a `reqwest::Client` object used to send the API request.

# Returns

A `Result` object that contains either a `GeocoderResult` struct representing the latitude and longitude coordinates of the queried location, or an `AppError` if there was an error during the API call or if the API key is missing.


*/

async fn call_geocoder_api(
    query_string: String,
    client: &Client,
) -> Result<GeocoderResult, AppError> {
    if let Ok(key) = dotenv::var("GEOCODER_API_KEY") {
        let encoded_query_string = encode(&query_string);
        let request_url = format!(
            "https://forward-reverse-geocoding.p.rapidapi.com/v1/search?q={queryString}",
            queryString = encoded_query_string
        );

        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("x-rapidapi-key"),
            HeaderValue::from_str(key.as_str())?,
        );
        headers.insert(
            HeaderName::from_static("x-rapidapi-host"),
            HeaderValue::from_static("forward-reverse-geocoding.p.rapidapi.com"),
        );

        let response = client.get(request_url).headers(headers).send().await?;
        let result_vec = response.json::<Vec<GeocoderResult>>().await?;
        result_vec
            .into_iter()
            .next()
            .ok_or(AppError(anyhow!("Failed to obtain valid geocoding result")))
    } else {
        Err(AppError(anyhow!("Missing geocoder API key")))
    }
}

#[debug_handler]
pub async fn get_latlong(Path(query_string): Path<String>) -> impl IntoResponse {
    let result = call_geocoder_api(query_string, &CLIENT).await;
    result
}
