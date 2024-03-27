use std::env;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Router,
};
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    tracing::debug!("connecting to redis");
    let manager = RedisConnectionManager::new("redis://127.0.0.1").unwrap();
    let pool = bb8::Pool::builder().build(manager).await.unwrap();

    {
        // ping the database before starting
        let mut conn = pool.get().await.unwrap();
        conn.set::<&str, &str, ()>("foo", "bar").await.unwrap();
        let result: String = conn.get("foo").await.unwrap();
        assert_eq!(result, "bar");
    }
    tracing::debug!("successfully connected to redis and pinged it");

    let app = Router::new()
        .route("/", get(index))
        .route("/weather/:city", get(city_weather))
        .route("/health", get(|| async { StatusCode::OK }))
        .with_state(pool);

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn index() -> &'static str {
    "Hello World"
}

type ConnectionPool = Pool<RedisConnectionManager>;

async fn city_weather(State(pool): State<ConnectionPool>, Path(city): Path<String>) -> String {
    let api_key = env::var("WEATHER_API_KEY").expect("WEATHER_API_KEY must be set");

    // check redis for the weather data
    let mut conn = pool.get().await.unwrap();
    let cached_weather = conn
        .get::<&std::string::String, Option<std::string::String>>(&city.to_lowercase())
        .await
        .unwrap();

    let body: WeatherResponse;

    match cached_weather {
        Some(cached) => {
            return format!(
                "Weather for {} was retrieved from cache: {:?}",
                city,
                serde_json::from_str::<WeatherResponse>(&cached).unwrap(),
            )
        }
        None => {
            let request_string = format!(
                "http://api.weatherapi.com/v1/current.json?key={}&q={}&aqi=no",
                api_key, city
            );

            let response = reqwest::get(request_string).await.unwrap();

            body = response.json::<WeatherResponse>().await.unwrap();

            let cached_body = conn
                .set::<&std::string::String, std::string::String, Option<std::string::String>>(
                    &body.location.name,
                    serde_json::to_string(&body).unwrap(),
                )
                .await
                .unwrap();

            match cached_body {
                Some(cached) => {
                    return format!(
                        "Weather for {} was cached as {:?}",
                        body.location.name, serde_json::to_string(&cached).unwrap()
                    )
                }
                None => return format!("Weather for {} was not cached", body.location.name),
            }
        }
    }
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
    localtime: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Current {
    last_updated_epoch: i64,
    last_updated: String,
    temp_c: f32,
    temp_f: f32,
}
