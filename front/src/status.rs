use dioxus::signals::{Signal, Writable};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum Server {
    Alive,
    Unstable,
    Dead,
}
impl Server {
    pub fn to_string(&mut self) -> String {
        match self {
            Self::Alive => String::from("Alive"),
            Self::Unstable => String::from("Unstable"),
            Self::Dead => String::from("Dead"),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum Coin {
    Tail,
    Head,
    Dead,
}
impl Coin {
    pub fn to_string(&mut self) -> String {
        match self {
            Self::Head => String::from("Head"),
            Self::Tail => String::from("Tail"),
            Self::Dead => String::from("Dead"),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ServerStatus {
    pub status: Server,
}
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CoinStatus {
    pub status: Coin,
}

pub async fn server_status_check(
    mut server_status: Signal<ServerStatus>,
    server_address: &String,
) -> ServerStatus {
    match reqwest::get(server_address).await {
        Ok(response) => match response.json::<ServerStatus>().await {
            Ok(_) => {
                *server_status.write() = ServerStatus {
                    status: Server::Alive,
                };
                ServerStatus {
                    status: Server::Alive,
                }
            }
            Err(err_val) => {
                *server_status.write() = ServerStatus {
                    status: Server::Dead,
                };
                log::info!("{}", err_val);
                ServerStatus {
                    status: Server::Dead,
                }
            }
        },
        Err(err_val) => {
            *server_status.write() = ServerStatus {
                status: Server::Dead,
            };
            log::info!("{}", err_val);
            ServerStatus {
                status: Server::Dead,
            }
        }
    }
}
pub async fn coin_status_check(server_address: &String) -> Option<CoinStatus> {
    match reqwest::get(format!("{}{}", server_address, "/coin")).await {
        Ok(response) => {
            match response.json::<CoinStatus>().await {
                Ok(coin_status)=> {
                    Some(coin_status)
                }
                Err(err_val) => {
                    log::error!("Error: Can't Deserialise -> {}", err_val);
                    None
                }
            }
        },
        Err(err_val) => {
            log::error!("Error: Response from Server -> {}", err_val);
            None
        },
    }
}
