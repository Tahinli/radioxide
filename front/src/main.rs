use std::mem::MaybeUninit;
use std::sync::Arc;
use std::time::Duration;
use ringbuf::{Consumer, HeapRb, Producer, SharedRb};
use tokio_with_wasm::tokio;
use tokio_tungstenite_wasm::*;
use futures_util::*;
use dioxus::prelude::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::Deserialize;
use tokio_with_wasm::tokio::time::sleep;

const SERVER_ADDRESS: &str = "https://tahinli.com.tr:2323";

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
async fn sound_stream(mut stream:WebSocketStream, mut producer: Producer<f32, Arc<SharedRb<f32, Vec<MaybeUninit<f32>>>>>) {
    while let Some(msg) = stream.next().await {
        match msg.unwrap().to_string().parse::<f32>() {
            Ok(sound_data) => {
                match producer.push(sound_data) {
                    Ok(_) => {},
                    Err(_) => {},
                }
            }
            Err(_) =>{}
        };
    }
    log::info!("Connection Lost Sir");
}
async fn start_listening() {
    log::info!("Trying Sir");
    let connect_addr = "ws://127.0.0.1:2424";
    let stream = tokio_tungstenite_wasm::connect(connect_addr).await.unwrap();
    let ring = HeapRb::<f32>::new(1000000);
    let (producer, consumer) = ring.split();
    let _sound_stream:Coroutine<()> = use_coroutine(|_| async move {
        sound_stream(stream, producer).await;
    });
    tokio_with_wasm::tokio::time::sleep(Duration::from_secs(1)).await;
    let _listen:Coroutine<()> = use_coroutine(|_| async move {
        listen(consumer).await;
    });
}
fn app() -> Element {
    rsx! {
        page_base {}
        div {
            button {
                onclick: move |_| start_listening(),
                "style":"width: 80px; height: 50px;",
                "Listen"
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

fn err_fn(err: cpal::StreamError) {
    eprintln!("Something Happened: {}", err);
}
pub async fn listen(mut consumer: Consumer<f32, Arc<SharedRb<f32, Vec<MaybeUninit<f32>>>>>) {

    log::info!("Hi");
    let host = cpal::default_host();
    let output_device = host.default_output_device().unwrap();
    let config:cpal::StreamConfig = output_device.default_output_config().unwrap().into();

    let output_data_fn = move |data: &mut [f32], _:&cpal::OutputCallbackInfo| {
        for sample in data {
            *sample = match consumer.pop() {
                Some(s) => s,
                None => {0.0},
            };
        }
    };


    let output_stream = output_device.build_output_stream(&config, output_data_fn, err_fn, None).unwrap();

    output_stream.play().unwrap();
    sleep(Duration::from_secs(100)).await;
    output_stream.pause().unwrap();
    // let host = cpal::default_host();
    // let devices = host.devices().unwrap();
    // for (_derive_index, device) in devices.enumerate() {
    //     log::info!("{:?}", device.name());
    // }
    // let device = host.default_output_device().unwrap();

    // let mut supported_config = device.supported_output_configs().unwrap();
    // let config = supported_config.next().unwrap().with_max_sample_rate();
    // log::info!("{:?}", config);
    // match config.sample_format() {
    //     cpal::SampleFormat::I8 => {log::info!("i8")},
    //     cpal::SampleFormat::I16 => {log::info!("i16")},
    //     //cpal::SampleFormat::I24 => {log::info!("i24")},
    //     cpal::SampleFormat::I32 => {log::info!("i32")},
    //     //cpal::SampleFormat::I48 => {log::info!("i48")},
    //     cpal::SampleFormat::I64 => {log::info!("i64")},
    //     cpal::SampleFormat::U8 => {log::info!("u8")},
    //     cpal::SampleFormat::U16 => {log::info!("u16")},
    //     //cpal::SampleFormat::U24 => {log::info!("u24")},
    //     cpal::SampleFormat::U32 => {log::info!("u32")},
    //     //cpal::SampleFormat::U48 => {log::info!("u48")},
    //     cpal::SampleFormat::U64 => {log::info!("u64")},
    //     cpal::SampleFormat::F32 => {log::info!("f32");
    //     run::<f32>(consumer, &device, &config.clone().into()).await.unwrap();},
    //     cpal::SampleFormat::F64 => {log::info!("f64")},
    //     sample_format => panic!("Unsupported sample format '{sample_format}'"),
    // }
    // let config:StreamConfig = config.into();
    // let stream = device.build_output_stream(
    //     &config, 
    //     move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            
    //         log::info!("{:?}", data);
    //         //I need to do something here, I think
    //     }, 
    //     move |_err| {
            
    //     }, 
    //     None).unwrap();
    
    // stream.play().unwrap();
    // tokio::time::sleep(Duration::from_secs(10)).await;
    // stream.pause().unwrap();

    
}
fn server_status_renderer() -> Element {
    let server_check_time = 1_u64;
    let mut server_status = use_signal(move || ServerStatus{status:Server::Unstable,});
    let mut server_status_watchdog = use_signal(move|| false);
    let mut server_status_unstable = use_signal(move|| false);
    let _server_status_task:Coroutine<()> = use_coroutine(|_| async move {
        loop {
            tokio::time::sleep(Duration::from_secs(server_check_time)).await;
            *server_status_watchdog.write() = true;
            *server_status.write() = server_status_check(server_status).await;
            *server_status_watchdog.write() = false;
        };
    });
    let _server_status_watchdog_timer:Coroutine<()> = use_coroutine(|_| async move {
            let mut watchdog_counter = 0_i8;
            loop {  
                tokio::time::sleep(Duration::from_secs(2*server_check_time+1)).await;
                if !server_status_watchdog() {
                    *server_status_unstable.write() = false;
                }
                if server_status_watchdog() {
                    for _i in 0..5 {
                        tokio::time::sleep(Duration::from_secs(1)).await;
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