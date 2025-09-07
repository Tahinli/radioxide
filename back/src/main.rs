use axum_server::tls_rustls::RustlsConfig;
use back::{routing, streaming, utils::get_config, AppState};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    let relay_config = get_config().await;
    let rustls_config =
        RustlsConfig::from_pem_file("certificates/fullchain.pem", "certificates/privkey.pem")
            .await
            .unwrap();
    let state = AppState {};
    let app = routing::routing(axum::extract::State(state)).await;
    let addr = SocketAddr::from(
        relay_config
            .axum_address
            .clone()
            .parse::<SocketAddr>()
            .unwrap(),
    );
    println!(
        "\n\n\tOn Air -> http://{}\n\n",
        relay_config.axum_address.clone()
    );
    tokio::spawn(streaming::start(relay_config));
    axum_server::bind_rustls(addr, rustls_config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
