use std::net::IpAddr;

use serde::{Deserialize, Serialize};

pub mod routing;
pub mod streaming;
pub mod utils;

#[derive(Debug, Clone)]
pub struct Config {
    pub axum_address: String,
    pub listener_address: String,
    pub streamer_address: String,
    pub latency: u16,
    pub tls: bool,
}

#[derive(Debug, Clone)]
pub struct AppState {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Streamer {
    ip: IpAddr,
    port: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Listener {
    ip: IpAddr,
    port: u16,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum ServerStatus {
    Alive,
    Unstable,
    Dead,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum CoinStatus {
    Tail,
    Head,
}
