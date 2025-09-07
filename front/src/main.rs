use dioxus::prelude::*;
use serde::Deserialize;

const SERVER_ADDRESS: &str = "http://localhost:2323";
#[derive(Debug, Clone, PartialEq, Deserialize)]
struct ServerStatus{
    status: String,
}
#[derive(Debug, Clone, PartialEq, Deserialize)]
struct CoinStatus{
    status: String,
}

fn main() {
    println!("Hello, world!");
    launch(app);
}

async fn server_status_check() ->Result<ServerStatus, reqwest::Error> {
    Ok(reqwest::get(SERVER_ADDRESS).await.unwrap().json::<ServerStatus>().await.unwrap())
}
async fn coin_status_check() -> Result<CoinStatus, reqwest::Error> {
    Ok(reqwest::get(SERVER_ADDRESS).await.unwrap().json::<CoinStatus>().await.unwrap())
}

fn app() -> Element {
    let server_status = use_resource(move || server_status_check());
    match &*server_status.value().read() {
        Some(Ok(server_status)) => {
            rsx! {
                { page_base() }
                h5 {
                    ShowServerStatus { server_status: server_status.clone() }
                }
            }
            
        }
        Some(Err(_)) => {
            rsx! {
                { page_base() }
                div {
                    "A"
                }
            }
        }
        None => {
            rsx! {
                { page_base() }
                h5 {
                    "Server Status: Dead"
                }
            }
        }
    }
}

fn page_base() ->Element {
    rsx! {
        h1 {
            "Radioxide"
        }
        div {
            div {
                class: "flex items-center",
                span {
                    button { 
                        "style":"width: 70px; height: 40px;",
                        "Coin Flip"
                    }
                }
                span { "   " },
                span {  }
            }
            
        }
    }
}

#[component]
fn ShowServerStatus(server_status: ServerStatus) -> Element {
    rsx! {
        div {
            div { class: "flex items-center",
            span { "Server Status: " }
            span { { server_status.status } }
            }
        }
    }
}
#[component]
fn ShowCoinStatus(coin_status: CoinStatus) -> Element {
    rsx! {
        div {
            div { class: "flex items-center",
            span { "Coin Status: " }
            span { { coin_status.status } }
            }
        }
    }
}