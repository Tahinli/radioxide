use crate::{AppState, ServerStatus, CoinStatus, streaming};
use axum::{body::Body, extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use tower_http::cors::CorsLayer;
use rand::prelude::*;

pub async fn routing(State(state): State<AppState>) -> Router {
    Router::new()
    .route("/", get(alive))
    .route("/coin", get(flip_coin))
    .route("/stream", get(stream))
    .layer(CorsLayer::permissive())
    .with_state(state.clone())
}

async fn alive() -> impl IntoResponse {
    let alive_json = serde_json::json!({
        "status":ServerStatus::Alive,
    });
    println!("{}", alive_json);
    (StatusCode::OK, Json(alive_json))
}

async fn flip_coin() -> impl IntoResponse {
    let mut rng = rand::thread_rng();
    let random:f64 = rng.gen();
    let mut flip_status = CoinStatus::Tail;
    if random > 0.5 {
        flip_status = CoinStatus::Head;
    }
    let coin_json = serde_json::json!({
        "status":flip_status,
    });
    println!("{}", coin_json);
    (StatusCode::OK, Json(coin_json))
}

#[axum::debug_handler]
async fn stream() -> impl IntoResponse {
    println!("Stream");
    streaming::start().await;
    let file = File::open("audios/audio.mp3").await.unwrap();
    let stream = ReaderStream::new(file);
    Body::from_stream(stream)
}