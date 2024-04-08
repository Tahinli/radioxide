use std::net::IpAddr;

use serde::{Deserialize, Serialize};

pub mod routing;
pub mod streaming;
pub mod utils;

#[derive(Debug, Clone)]
pub struct AppState {}

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
