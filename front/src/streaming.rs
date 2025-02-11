use brotli::DecompressorWriter;
use dioxus::{
    prelude::spawn,
    signals::{Signal, Writable},
};
use futures_util::StreamExt;
use ringbuf::{HeapRb, Producer, SharedRb};
use std::{io::Write, mem::MaybeUninit, sync::Arc};

use crate::{listening::listen_podcast, BUFFER_LENGTH};

pub async fn start_listening(
    mut is_maintaining: Signal<(bool, bool)>,
    mut is_listening: Signal<bool>,
) {
    if is_listening() {
        log::info!("Trying Sir");
        let connect_addr = "ws://127.0.0.1:2424";

        let ws_stream: tokio_tungstenite_wasm::WebSocketStream;
        match tokio_tungstenite_wasm::connect(connect_addr).await {
            Ok(ws_stream_connected) => ws_stream = ws_stream_connected,
            Err(_) => {
                is_listening.set(false);
                return;
            }
        }
        is_maintaining.set((true, true));
        let ring = HeapRb::<f32>::new(BUFFER_LENGTH);
        let (producer, consumer) = ring.split();
        let _sound_stream_task = spawn({
            async move {
                sound_stream(is_listening, ws_stream, producer).await;
                is_listening.set(false);
                is_maintaining.set((false, is_maintaining().1));
            }
        });
        let _listen_podcast_task = spawn({
            async move {
                listen_podcast(is_listening, consumer).await;
                is_listening.set(false);
                //stream_producer.send("Disconnect ME".into()).await.unwrap();
                is_maintaining.set((is_maintaining().0, false));
            }
        });
    }
}

pub async fn sound_stream(
    is_listening: Signal<bool>,
    mut ws_stream: tokio_tungstenite_wasm::WebSocketStream,
    mut producer: Producer<f32, Arc<SharedRb<f32, Vec<MaybeUninit<f32>>>>>,
) {
    log::info!("Attention! We need cables");

    while let Some(message_with_question) = ws_stream.next().await {
        if is_listening() {
            //log::info!("{}", message_with_question.unwrap().len());
            let data: Vec<u8> = message_with_question.unwrap().into();
            let mut decompression_writer = DecompressorWriter::new(vec![], BUFFER_LENGTH);
            if let Err(err_val) = decompression_writer.write_all(&data) {
                log::error!("Error: Decompression | {}", err_val);
            }
            let uncompressed_data = match decompression_writer.into_inner() {
                Ok(healthy_packet) => healthy_packet,
                Err(unhealthy_packet) => {
                    log::warn!("Warning: Unhealthy Packet | {}", unhealthy_packet.len());
                    unhealthy_packet
                }
            };
            log::info!("{}", uncompressed_data.len());
            let data = String::from_utf8(uncompressed_data).unwrap();
            let mut datum_parsed: Vec<char> = vec![];
            let mut data_parsed: Vec<String> = vec![];
            for char in data.chars() {
                if char == '+' || char == '-' {
                    data_parsed.push(datum_parsed.iter().collect());
                    datum_parsed.clear();
                }
                datum_parsed.push(char);
                if data.len() > 2 {
                    if char == '+' || char == '-' {
                        datum_parsed.push('0');
                        datum_parsed.push('.');
                    }
                }
            }

            for single_data in data_parsed {
                let sample = match single_data.parse::<f32>() {
                    Ok(sample) => sample,
                    Err(_) => 0.0,
                };
                if let Err(_) = producer.push(sample) {}
            }
        } else {
            break;
        }
    }

    log::info!("Connection Lost Sir");
}
