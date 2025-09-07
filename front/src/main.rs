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
    Ok(reqwest::get(format!("{}{}", SERVER_ADDRESS, "/coin")).await.unwrap().json::<CoinStatus>().await.unwrap())
}
fn app() -> Element {
    rsx! {
        { page_base() }
        { coin_status_renderer() }
        { server_status_renderer() }
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
                }
            }
            
        }
    }
}

fn server_status_renderer() -> Element {
    let server_status = use_resource(move || server_status_check());
    match &*server_status.value().read() {
        Some(Ok(server_status)) => {
            rsx! {
                h5 {
                    ShowServerStatus { server_status: server_status.clone() }
                }
            }
            
        }
        Some(Err(err_val)) => {
            rsx! {
                h5 {
                    "Server Status: "
                    { err_val.to_string() }
                }
            }
        }
        None => {
            rsx! {
                h5 {
                    "Server Status: Dead"
                }
            }
        }
    }
}
fn coin_status_renderer() -> Element {
    let mut coin_response = use_resource(move || coin_status_check());
    match &*coin_response.value().read() {
        Some(Ok(coin_status)) => {
            rsx! {
                button { 
                    onclick: move |_| coin_response.restart(),
                    "style":"width: 70px; height: 40px;",
                    "Coin Flip"
                }
                ShowCoinStatus{ coin_status: coin_status.clone() }
            }
        }
        Some(Err(err_val)) => {
            rsx! {
                div {
                    "Coin Status: "
                    { err_val.to_string() }
                }
            }
        }
        None => {
            rsx! {
                div {
                    "Coin Status: None"
                }
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