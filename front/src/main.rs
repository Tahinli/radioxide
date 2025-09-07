use std::time::Duration;
use async_std::task;
use dioxus::prelude::*;
use serde::Deserialize;

const SERVER_ADDRESS: &str = "https://localhost:2323";

static SERVER_STATUS:GlobalSignal<ServerStatus> = Signal::global(move || ServerStatus {status:"Alive".to_string(),});
static SERVER_STATUS_WATCHDOG:GlobalSignal<bool> = Signal::global(move || false);
static SERVER_STATUS_LOOSING:GlobalSignal<bool> = Signal::global(move || false);
static SERVER_STATUS_IS_DEAD:GlobalSignal<bool> = Signal::global(move || false);

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

async fn server_status_check() ->ServerStatus {
    match reqwest::get(SERVER_ADDRESS).await {
        Ok(response) => {
            *SERVER_STATUS_WATCHDOG.write() = false;
            match response.json::<ServerStatus>().await {
                Ok(server_status) => {
                    *SERVER_STATUS_LOOSING.write() = false;
                    server_status
                }
                Err(err_val) => {
                    *SERVER_STATUS_LOOSING.write() = true;
                    ServerStatus{status:err_val.to_string(),}
                }
            }
        }
        Err(err_val) => {
            *SERVER_STATUS_LOOSING.write() = true;
            ServerStatus{status:err_val.to_string(),}
        }
    }
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
    let server_check_time = 1_u64;
    let _server_status_task:Coroutine<()> = use_coroutine(|_| async move {
        loop {
            task::sleep(Duration::from_secs(server_check_time)).await;
            *SERVER_STATUS_WATCHDOG.write() = true;
            *SERVER_STATUS.write() = server_status_check().await;
            /*match server_status_check().await {
                Ok(status) => {
                    server_status_watchdog.set(false);
                    server_status.set(status);
                }
                Err(err_val) => {
                    server_status.set(ServerStatus {status:err_val.to_string(),});
                }
            }*/
            
        };
    });
    let _server_status_watchdog_timer:Coroutine<()> = use_coroutine(|_| async move {
            let mut is_loosing_counter = 0_i8;
            loop {  
                task::sleep(Duration::from_secs(2*server_check_time+1)).await;
                if !SERVER_STATUS_WATCHDOG() {
                    *SERVER_STATUS_LOOSING.write() = false;
                }
                if SERVER_STATUS_WATCHDOG() {
                    for _i in 0..5 {
                        task::sleep(Duration::from_secs(1)).await;
                        if SERVER_STATUS_WATCHDOG() {
                            is_loosing_counter += 1;
                        }
                    }
                    
                }
                if is_loosing_counter > 4 {
                    *SERVER_STATUS_LOOSING.write() = true;
                }
                is_loosing_counter = 0;
        }
    });
    rsx! {
        if SERVER_STATUS_LOOSING() && SERVER_STATUS_WATCHDOG() {
            {*SERVER_STATUS_IS_DEAD.write() = true}
            ShowServerStatus {server_status:ServerStatus{status:"Dead".to_string(),}}
        }
        else {
            ShowServerStatus {server_status:SERVER_STATUS()}        
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
                    Err(_) => {
                        is_loading.set(false);
                    }
                }
            }
        });
    };
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