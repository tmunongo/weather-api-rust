use std::env;

use axum::{
    extract::Path, http::StatusCode, routing::get, Router
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(index))
        .route("/weather/:city", get(city_weather))
        .route("/health", get(|| async { StatusCode::OK }));

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn index() -> &'static str {
    "Hello World"
}

async fn city_weather(Path(city): Path<String>) -> String {
    let api_key = env::var("WEATHER_API_KEY").expect("WEATHER_API_KEY must be set");

    let request_string = format!("http://api.weatherapi.com/v1/current.json?key={}&q={}&aqi=no", api_key, city);

    let response = reqwest::get(request_string)
    .await
    .unwrap();

    let body: WeatherResponse = response.json::<WeatherResponse>().await.unwrap();

    format!("Weather for {:?}", body)
}

#[derive(Serialize, Deserialize, Debug)]
struct WeatherResponse {
    location: Location,
    current: Current,
}

#[derive(Debug, Deserialize, Serialize)]
struct Location {
    name: String,
    region: String,
    country: String,
    lat: f64,
    lon: f64,
    tz_id: String,
    localtime_epoch: i64,
    localtime: String
}

#[derive(Debug, Deserialize, Serialize)]
struct Current {
    last_updated_epoch: i64,
    last_updated: String,
    temp_c: f32,
    temp_f: f32,
}