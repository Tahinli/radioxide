use crate::status::{
    coin_status_check, server_status_check, Coin, CoinStatus, Server, ServerStatus,
};
use dioxus::prelude::*;
use std::time::Duration;

#[component]
pub fn server_status_renderer(server_address: String) -> Element {
    let server_check_time = 1_u64;
    let mut server_status = use_signal(move || ServerStatus {
        status: Server::Unstable,
    });
    let mut server_status_watchdog = use_signal(move || false);
    let mut server_status_unstable = use_signal(move || false);
    let _server_status_task: Coroutine<()> = use_coroutine(|_| async move {
        loop {
            tokio_with_wasm::tokio::time::sleep(Duration::from_secs(server_check_time)).await;
            *server_status_watchdog.write() = true;
            *server_status.write() = server_status_check(server_status, &server_address).await;
            *server_status_watchdog.write() = false;
        }
    });
    let _server_status_watchdog_timer: Coroutine<()> = use_coroutine(|_| async move {
        let mut watchdog_counter = 0_i8;
        loop {
            tokio_with_wasm::tokio::time::sleep(Duration::from_secs(2 * server_check_time + 1))
                .await;
            if !server_status_watchdog() {
                *server_status_unstable.write() = false;
            }
            if server_status_watchdog() {
                for _i in 0..5 {
                    tokio_with_wasm::tokio::time::sleep(Duration::from_secs(1)).await;
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

#[component]
pub fn coin_status_renderer(server_address: String) -> Element {
    let is_loading = use_signal(|| false);
    let coin_result = use_signal(|| CoinStatus { status: Coin::Head });
    let call_coin = move |_| {
        spawn({
            to_owned![is_loading];
            to_owned![coin_result];
            to_owned![server_address];
            is_loading.set(true);
            async move {
                match coin_status_check(&server_address).await {
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
