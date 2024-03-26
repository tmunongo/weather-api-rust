use axum::{
    extract::Path, http::StatusCode, routing::get, Router
};
use tokio::net::TcpListener;
// use serde::{Serialize, Deserialize};

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
    format!("Weather for {}", city)
}
