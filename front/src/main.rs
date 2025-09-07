use dioxus::prelude::*;
use serde::Deserialize;

const SERVER_ADDRESS: &str = "https://localhost:2323";
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
    wasm_logger::init(wasm_logger::Config::default());
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
        page_base {}
        coin_status_renderer {}
        server_status_renderer {}
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
    let is_loading = use_signal(|| false);
    let coin_result = use_signal(|| CoinStatus{status:"None".to_string(),});
    let call_coin = move |_| {
        spawn({
            to_owned![is_loading];
            to_owned![coin_result];
            is_loading.set(true);
            async move {
                match coin_status_check().await {
                    Ok(coin_status) => {
                        is_loading.set(false);
                        coin_result.set(coin_status);
                    }
                    Err(err_val) => {
                        is_loading.set(false);
                        log::info!("{}", err_val);
                    }
                }
            }
        });
    };
    log::info!("{}", is_loading);
    rsx! {
        div {
            button {
                disabled: is_loading(),
                onclick: call_coin,
                "style":"width: 80px; height: 50px;",
                if is_loading() {
                    "Loading"
                }else {
                    "Coin Flip"
                }
            }
            div {
                ShowCoinStatus{ coin_status: coin_result().clone() }
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