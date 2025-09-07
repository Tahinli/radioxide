use crate::AppState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use tower_http::cors::CorsLayer;


pub async fn routing(State(state): State<AppState>) -> Router{
    Router::new()
    .route("/", get(alive))
    .layer(CorsLayer::permissive())
    .with_state(state.clone())
}

async fn alive() -> impl IntoResponse{
    let alive_json = serde_json::json!({
        "status":"alive",
    });
    println!("Alive");
    (StatusCode::OK, Json(alive_json))
    
}