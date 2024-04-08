use axum_server::tls_rustls::RustlsConfig;
use back::{routing, streaming, AppState};
use std::{env, net::SocketAddr};

fn take_args() -> String {
    let mut bind_address: String = String::new();
    for element in env::args() {
        bind_address = element
    }
    println!("\n\n\tOn Air -> http://{}\n\n", bind_address);
    bind_address
}

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    let config =
        RustlsConfig::from_pem_file("certificates/fullchain.pem", "certificates/privkey.pem")
            .await
            .unwrap();
    let state = AppState {};
    let app = routing::routing(axum::extract::State(state)).await;
    let addr = SocketAddr::from(take_args().parse::<SocketAddr>().unwrap());
    tokio::spawn(streaming::start());
    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
