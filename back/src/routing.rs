use crate::AppState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use tower_http::cors::CorsLayer;
use rand::prelude::*;


pub async fn routing(State(state): State<AppState>) -> Router {
    Router::new()
    .route("/", get(alive))
    .route("/coin", get(flip_coin))
    .layer(CorsLayer::permissive())
    .with_state(state.clone())
}

async fn alive() -> impl IntoResponse {
    let alive_json = serde_json::json!({
        "status":"Alive",
    });
    println!("Alive");
    (StatusCode::OK, Json(alive_json))
}

async fn flip_coin() -> impl IntoResponse {
    let mut rng = rand::thread_rng();
    let random:f64 = rng.gen();
    let mut flip_status:String = "Tail".to_string();
    if random > 0.5 {
        flip_status = "Head".to_string();
    }
    let coin_json = serde_json::json!({
        "status":flip_status,
    });
    println!("Coin Flip");
    (StatusCode::OK, Json(coin_json))
}