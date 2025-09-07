use std::time::Duration;
use async_std::task;
use dioxus::prelude::*;
use serde::Deserialize;
const SERVER_ADDRESS: &str = "https://localhost:2323";

#[derive(Debug, Clone, PartialEq, Deserialize)]
enum Server{
    Alive,
    Unstable,
    Dead,
}
impl Server {
    fn to_string(&mut self) -> String{
        match self {
            Self::Alive => {String::from("Alive")},
            Self::Unstable => {String::from("Unstable")},
            Self::Dead => {String::from("Dead")},
        }
    }
}
#[derive(Debug, Clone, PartialEq, Deserialize)]
enum Coin{
    Tail,
    Head,
}
impl Coin {
    fn to_string(&mut self) -> String {
        match self {
            Self::Head => {String::from("Head")},
            Self::Tail => {String::from("Tail")},
        }
    }
}
#[derive(Debug, Clone, PartialEq, Deserialize)]
struct  ServerStatus {
    status:Server,
}
#[derive(Debug, Clone, PartialEq, Deserialize)]
struct CoinStatus {
    status:Coin,
}

fn main() {
    println!("Hello, world!");
    wasm_logger::init(wasm_logger::Config::default());
    launch(app);
}

async fn server_status_check(mut server_status:Signal<ServerStatus>) ->ServerStatus {
    match reqwest::get(SERVER_ADDRESS).await {
        Ok(response) => {
            match response.json::<ServerStatus>().await {
                Ok(_) => {
                    *server_status.write() = ServerStatus{status:Server::Alive,};
                    ServerStatus{status:Server::Alive,}
                }
                Err(err_val) => {
                    *server_status.write() = ServerStatus{status:Server::Dead,};
                    log::info!("{}", err_val);
                    ServerStatus{status:Server::Dead,}
                }
            }
        }
        Err(err_val) => {
            *server_status.write() = ServerStatus{status:Server::Dead,};
            log::info!("{}", err_val);
            ServerStatus{status:Server::Dead,}
        }
    }
}
async fn coin_status_check() -> Result<CoinStatus, reqwest::Error> {
    Ok(reqwest::get(format!("{}{}", SERVER_ADDRESS, "/coin")).await.unwrap().json::<CoinStatus>().await.unwrap())
}
fn app() -> Element {
    rsx! {
        page_base {}
        div {
            audio{  
                    src:"https://radioxide.tahinli.com.tr:2323/stream",
                    controls:true, 
                    autoplay: true,
                    muted:false,
                    r#loop:true,
            }
        }
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
    let mut server_status = use_signal(move || ServerStatus{status:Server::Unstable,});
    let mut server_status_watchdog = use_signal(move|| false);
    let mut server_status_unstable = use_signal(move|| false);
    let _server_status_task:Coroutine<()> = use_coroutine(|_| async move {
        loop {
            task::sleep(Duration::from_secs(server_check_time)).await;
            *server_status_watchdog.write() = true;
            *server_status.write() = server_status_check(server_status).await;
            *server_status_watchdog.write() = false;
        };
    });
    let _server_status_watchdog_timer:Coroutine<()> = use_coroutine(|_| async move {
            let mut watchdog_counter = 0_i8;
            loop {  
                task::sleep(Duration::from_secs(2*server_check_time+1)).await;
                if !server_status_watchdog() {
                    *server_status_unstable.write() = false;
                }
                if server_status_watchdog() {
                    for _i in 0..5 {
                        task::sleep(Duration::from_secs(1)).await;
                        if server_status_watchdog() {
                            watchdog_counter += 1;
                        }
                    }
                    
                }
                if watchdog_counter > 4 {
                    *server_status_unstable.write() = true;
                }
                watchdog_counter = 0;
        }
    });
    rsx! {
        if server_status_unstable() && server_status_watchdog() {
            ShowServerStatus {server_status:ServerStatus{status:Server::Dead,}}
        }
        else {
            ShowServerStatus {server_status:server_status()}        
        }
    }
    
}
fn coin_status_renderer() -> Element {
    let is_loading = use_signal(|| false);
    let coin_result = use_signal(|| CoinStatus{status:Coin::Head,});
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
            span { { server_status.status.to_string() } }
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
            span { { coin_status.status.to_string() } }
            }
        }
    }
}