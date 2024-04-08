use std::{mem::MaybeUninit, sync::Arc, time::Duration};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use dioxus::{
    prelude::spawn,
    signals::{Signal, Writable},
};
use futures_util::{stream::SplitStream, SinkExt, StreamExt};

use ringbuf::{Consumer, HeapRb, Producer, SharedRb};
use tokio_tungstenite_wasm::WebSocketStream;

static BUFFER_LIMIT: usize = 800000;
static BUFFER_LENGTH: usize = 1000000;

pub async fn start_listening(
    mut is_maintaining: Signal<(bool, bool)>,
    mut is_listening: Signal<bool>,
) {
    if is_listening() {
        log::info!("Trying Sir");
        let connect_addr = "ws://192.168.1.2:2424";
        let (mut stream_producer, stream_consumer);
        match tokio_tungstenite_wasm::connect(connect_addr).await {
            Ok(ws_stream) => (stream_producer, stream_consumer) = ws_stream.split(),
            Err(_) => {
                is_listening.set(false);
                return;
            }
        }
        is_maintaining.set((true, true));
        let ring = HeapRb::<f32>::new(BUFFER_LENGTH);
        let (producer, consumer) = ring.split();
        spawn({
            async move {
                sound_stream(is_listening, stream_consumer, producer).await;
                is_listening.set(false);
                is_maintaining.set((false, is_maintaining().1));
            }
        });
        spawn({
            async move {
                listen_podcast(is_listening, consumer).await;
                is_listening.set(false);
                //stream_producer.send("Disconnect ME".into()).await.unwrap();
                stream_producer.close().await.unwrap();
                //buffer time waiting actually
                tokio_with_wasm::tokio::time::sleep(Duration::from_secs(2)).await;
                log::info!("{:#?}", is_maintaining());
                is_maintaining.set((is_maintaining().0, false));
                log::info!("pod{:#?}", is_maintaining());
            }
        });
    }
}

pub async fn sound_stream(
    is_listening: Signal<bool>,
    mut stream_consumer: SplitStream<WebSocketStream>,
    mut producer: Producer<f32, Arc<SharedRb<f32, Vec<MaybeUninit<f32>>>>>,
) {
    log::info!("Attention! We need cables");
    
    while let Some(msg) = stream_consumer.next().await {
        if is_listening() {
            let data = String::from_utf8(msg.unwrap().into()).unwrap();
            let data_parsed: Vec<&str> = data.split("#").collect();
            for element in data_parsed {
                let single_data: f32 = match element.parse() {
                    Ok(single) => single,
                    Err(_) => 0.0,
                };
                if let Err(_) = producer.push(single_data) {}
            }
        } else {
            break;
        }
    }

    log::info!("Connection Lost Sir");
}
async fn listen_podcast(
    is_listening: Signal<bool>,
    mut consumer: Consumer<f32, Arc<SharedRb<f32, Vec<MaybeUninit<f32>>>>>,
) {
    log::info!("Attention! Show must start!");
    let host = cpal::default_host();
    let output_device = host.default_output_device().unwrap();
    let config: cpal::StreamConfig = output_device.default_output_config().unwrap().into();

    let output_data_fn = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        if consumer.len() > BUFFER_LIMIT {
            consumer.clear();
            log::error!("Slow Consumer: DROPPED ALL Packets");
        }
        for sample in data {
            *sample = match consumer.pop() {
                Some(s) => s,
                None => 0.0,
            };
        }
    };

    let output_stream = output_device
        .build_output_stream(&config, output_data_fn, err_fn, None)
        .unwrap();

    output_stream.play().unwrap();

    while is_listening() {
        tokio_with_wasm::tokio::time::sleep(Duration::from_secs(1)).await;
    }

    output_stream.pause().unwrap();
    log::info!("Attention! Time to turn home!");
}
fn err_fn(err: cpal::StreamError) {
    eprintln!("Something Happened: {}", err);
}
