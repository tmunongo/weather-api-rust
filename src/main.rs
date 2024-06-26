use std::env;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use dotenvy::dotenv;
use redis::{AsyncCommands, FromRedisValue, RedisResult};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    dotenv().expect("Failed to load .env file");

    let redis_url = env::var("REDIS_URL").expect("REDIS URL must be set!");

    tracing::debug!("connecting to redis");
    let manager = RedisConnectionManager::new(redis_url).unwrap();
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

async fn city_weather(
    State(pool): State<ConnectionPool>,
    Path(city): Path<String>,
) -> Result<Json<WeatherResponse>, String> {
    let api_key = env::var("WEATHER_API_KEY").expect("WEATHER_API_KEY must be set!");

    // check redis for the weather data
    let mut conn = pool.get().await.unwrap();
    let cached_weather: Option<WeatherResponse> = conn
        .get::<String, Option<WeatherResponse>>(city.as_str().to_owned())
        .await
        .unwrap();

    let body: WeatherResponse;

    if cached_weather.is_some() {
        return Ok(Json(cached_weather.unwrap()));
    } else {
        let request_string = format!(
            "https://api.weatherapi.com/v1/current.json?key={}&q={}&aqi=no",
            api_key, city
        );

        let response = reqwest::get(request_string).await.unwrap();

        body = response.json::<WeatherResponse>().await.unwrap();

        let cached_body: Option<WeatherResponse> = conn
            .set_ex(
                &body.location.name.to_ascii_lowercase(),
                serde_json::to_string(&body).unwrap(),
                3600,
            )
            .await
            .unwrap();

        match cached_body {
            Some(_) => Ok(Json(body.clone())),
            None => Err(format!("Weather for {} was not cached", body.location.name)),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct WeatherResponse {
    location: Location,
    current: Current,
}

impl FromRedisValue for WeatherResponse {
    fn from_redis_value(v: &redis::Value) -> RedisResult<Self> {
        match v {
            redis::Value::Data(data) => {
                // Assuming the data stored in Redis is JSON representing a WeatherResponse
                let json_str = std::str::from_utf8(data).expect("Invalid UTF-8 data in Redis");
                let weather_response: WeatherResponse =
                    serde_json::from_str(json_str).expect("Failed to deserialize JSON");

                Ok(weather_response)
            }
            _ => Err(redis::RedisError::from((
                redis::ErrorKind::TypeError,
                "Invalid Redis value type for WeatherResponse",
            ))),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
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

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Current {
    last_updated_epoch: i64,
    last_updated: String,
    temp_c: f32,
    temp_f: f32,
}
