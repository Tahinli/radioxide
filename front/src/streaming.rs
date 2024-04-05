use std::{mem::MaybeUninit, sync::Arc, time::Duration};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use dioxus::hooks::{use_coroutine, Coroutine};
use futures_util::StreamExt;
use ringbuf::{Consumer, HeapRb, Producer, SharedRb};
use tokio_tungstenite_wasm::WebSocketStream;
use tokio_with_wasm::tokio::time::sleep;

pub async fn start_listening() {
    log::info!("Trying Sir");
    let connect_addr = "ws://192.168.1.2:2424";
    let stream = tokio_tungstenite_wasm::connect(connect_addr).await.unwrap();
    let ring = HeapRb::<f32>::new(1000000);
    let (producer, consumer) = ring.split();
    let _sound_stream: Coroutine<()> = use_coroutine(|_| async move {
        sound_stream(stream, producer).await;
    });
    tokio_with_wasm::tokio::time::sleep(Duration::from_secs(1)).await;
    let _listen: Coroutine<()> = use_coroutine(|_| async move {
        listen(consumer).await;
    });
}

async fn sound_stream(
    mut stream: WebSocketStream,
    mut producer: Producer<f32, Arc<SharedRb<f32, Vec<MaybeUninit<f32>>>>>,
) {
    while let Some(msg) = stream.next().await {
        let data = String::from_utf8(msg.unwrap().into()).unwrap();
        let data_parsed:Vec<&str> = data.split("#").collect();
        //let mut sound_data:Vec<f32> = vec![];
        for element in data_parsed {
            let single_data:f32 = match element.parse() {
                Ok(single) => single,
                Err(_) => 0.0,
            };
            producer.push(single_data).unwrap();
        }
    }
    // while let Some(msg) = stream.next().await {
    //     match msg.unwrap().to_string().parse::<f32>() {
    //         Ok(sound_data) => match producer.push(sound_data) {
    //             Ok(_) => {}
    //             Err(_) => {}
    //         },
    //         Err(_) => {}
    //     };
    // }
    log::info!("Connection Lost Sir");
}
async fn listen(mut consumer: Consumer<f32, Arc<SharedRb<f32, Vec<MaybeUninit<f32>>>>>) {
    log::info!("Hi");
    let host = cpal::default_host();
    let output_device = host.default_output_device().unwrap();
    let config: cpal::StreamConfig = output_device.default_output_config().unwrap().into();

    let output_data_fn = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
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
fn err_fn(err: cpal::StreamError) {
    eprintln!("Something Happened: {}", err);
}
