use back::{routing, AppState};
use std::env::{self};

fn take_args() -> String{
    let mut bind_address:String = String::new();
    for element in env::args(){
        bind_address = element
    }
    println!("\n\n\tOn Air -> http://{}\n\n", bind_address);
    bind_address
}

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    let state = AppState{

    };
    let app = routing::routing(axum::extract::State(state)).await;
    let listener = tokio::net::TcpListener::bind(take_args()).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
