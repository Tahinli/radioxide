use std::{mem::MaybeUninit, sync::Arc, time::Duration};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use dioxus::signals::Signal;
use ringbuf::{Consumer, SharedRb};

use crate::BUFFER_LIMIT;

pub async fn listen_podcast(
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
