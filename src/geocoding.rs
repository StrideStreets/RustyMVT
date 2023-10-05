use axum::{extract::Path, response::IntoResponse, Json};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue, ACCESS_CONTROL_ALLOW_ORIGIN},
    Client,
};
use serde::{de, Deserialize, Deserializer, Serialize};
use serde_json::Value;

use axum_macros::debug_handler;

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

async fn call_geocoder_api(query_string: String) -> GeocoderResult {
    let api_key: &'static str = dotenv!("GEOCODER_API_KEY");
    let request_url = format!(
        "https://forward-reverse-geocoding.p.rapidapi.com/v1/search?q={queryString}",
        queryString = &query_string
    );

    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("x-rapidapi-key"),
        HeaderValue::from_static(api_key),
    );
    headers.insert(
        HeaderName::from_static("x-rapidapi-host"),
        HeaderValue::from_static("forward-reverse-geocoding.p.rapidapi.com"),
    );

    let response = Client::new()
        .get(request_url)
        .headers(headers)
        .send()
        .await
        .unwrap();

    response
        .json::<Vec<GeocoderResult>>()
        .await
        .unwrap()
        .into_iter()
        .next()
        .unwrap()
}

#[debug_handler]
pub async fn get_latlong(Path(query_string): Path<String>) -> impl IntoResponse {
    let response: GeocoderResult = call_geocoder_api(query_string).await;
    response
}
