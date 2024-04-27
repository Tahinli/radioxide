use std::{io::Write, sync::Arc, time::Duration};

use brotli::CompressorWriter;
use futures_util::SinkExt;
use ringbuf::HeapRb;
use tokio::{
    sync::broadcast::{channel, Receiver, Sender},
    task::JoinHandle,
};
use tokio_tungstenite::tungstenite::Message;

use crate::{Config, BUFFER_LENGTH};
const MAX_TOLERATED_MESSAGE_COUNT: usize = 10;

pub async fn connect(
    sound_stream_consumer: Receiver<f32>,
    streamer_config: Config,
    mut stop_connection_consumer: Receiver<bool>,
    connection_cleaning_status_producer: Sender<bool>,
) {
    let connect_addr = match streamer_config.tls {
        true => format!("wss://{}", streamer_config.address),
        false => format!("ws://{}", streamer_config.address),
    };

    if let Err(_) = stop_connection_consumer.try_recv() {
        let ws_stream;
        match streamer_config.tls {
            true => {
                let tls_client_config = rustls_platform_verifier::tls_config();
                let tls_connector =
                    tokio_tungstenite::Connector::Rustls(Arc::new(tls_client_config));

                match tokio_tungstenite::connect_async_tls_with_config(
                    connect_addr.clone(),
                    None,
                    false,
                    Some(tls_connector),
                )
                .await
                {
                    Ok(wss_stream_connected) => ws_stream = wss_stream_connected.0,
                    Err(_) => {
                        return;
                    }
                }
            }
            false => match tokio_tungstenite::connect_async(connect_addr.clone()).await {
                Ok(ws_stream_connected) => ws_stream = ws_stream_connected.0,
                Err(_) => {
                    return;
                }
            },
        }
        let (message_producer, message_consumer) = channel(BUFFER_LENGTH);
        println!("Connected to: {}", connect_addr);
        let message_organizer_task = tokio::spawn(message_organizer(
            message_producer,
            sound_stream_consumer,
            streamer_config.quality,
            streamer_config.latency,
        ));
        let stream_task = tokio::spawn(stream(ws_stream, message_consumer));
        tokio::spawn(status_checker(
            message_organizer_task,
            stream_task,
            stop_connection_consumer,
            connection_cleaning_status_producer,
        ));
    }
}

async fn message_organizer(
    message_producer: Sender<Message>,
    mut consumer: Receiver<f32>,
    quality: u8,
    latency: u16,
) {
    loop {
        let mut messages: Vec<u8> = Vec::new();
        let mut iteration = consumer.len();
        while iteration > 0 {
            iteration -= 1;
            match consumer.recv().await {
                Ok(single_data) => {
                    let ring = HeapRb::<u8>::new(BUFFER_LENGTH);
                    let (mut producer, mut consumer) = ring.split();
                    let mut charred: Vec<char> = single_data.to_string().chars().collect();
                    if charred[0] == '0' {
                        charred.insert(0, '+');
                    }
                    if charred.len() > 2 {
                        let _zero = charred.remove(1);
                        let _point = charred.remove(1);
                    }
                    charred.truncate(quality.into());
                    let mut single_data_packet: Vec<u8> = vec![];
                    for char in charred {
                        let char_packet = char.to_string().as_bytes().to_vec();
                        for byte in char_packet {
                            single_data_packet.push(byte);
                        }
                    }
                    for element in single_data_packet {
                        producer.push(element).unwrap();
                    }
                    while !consumer.is_empty() {
                        messages.push(consumer.pop().unwrap());
                    }
                }
                Err(_) => {}
            }
        }
        if !messages.is_empty() {
            let mut compression_writer = CompressorWriter::new(vec![], BUFFER_LENGTH, 4, 24);
            if let Err(err_val) = compression_writer.write_all(&messages) {
                eprintln!("Error: Compression | {}", err_val);
            }
            let compressed_messages = compression_writer.into_inner();
            // println!("Compressed Len {}", compressed_messages.len());
            // println!("UNCompressed Len {}", messages.len());
            match message_producer.send(compressed_messages.into()) {
                Ok(_) => {}
                Err(_) => {}
            }
            // println!(
            //     "Message Counter = {} | Receiver Count = {}",
            //     message_producer.len(),
            //     message_producer.receiver_count()
            // );
        }
        tokio::time::sleep(Duration::from_millis(latency.into())).await;
    }
}

async fn stream<T: futures_util::Sink<Message> + std::marker::Unpin>(
    mut ws_stream: T,
    mut message_consumer: Receiver<Message>,
) {
    while let Ok(message) = message_consumer.recv().await {
        if message_consumer.len() > MAX_TOLERATED_MESSAGE_COUNT {
            // println!(
            //     "{} Forced to Disconnect | Reason -> Slow Consumer",
            //     format!("{}:{}", listener.ip, listener.port)
            // );
            break;
        }
        match ws_stream.send(message).await {
            Ok(_) => {
                if let Err(_) = ws_stream.flush().await {
                    // println!(
                    //     "{} is Disconnected",
                    //     format!("{}:{}", listener.ip, listener.port)
                    // );
                    break;
                }
            }
            Err(_) => {
                // println!(
                //     "{} is Disconnected",
                //     format!("{}:{}", listener.ip, listener.port)
                // );
                break;
            }
        }
    }
}

async fn status_checker(
    message_organizer_task: JoinHandle<()>,
    stream_task: JoinHandle<()>,
    mut stop_connection_consumer: Receiver<bool>,
    connection_cleaning_status_producer: Sender<bool>,
) {
    let mut connection_cleaning_status_consumer = connection_cleaning_status_producer.subscribe();
    connection_cleaning_status_producer.send(true).unwrap();
    while let Err(_) = stop_connection_consumer.try_recv() {
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
    stream_task.abort();
    message_organizer_task.abort();
    while let Ok(_) = connection_cleaning_status_consumer.try_recv() {}
    drop(connection_cleaning_status_consumer);
    drop(connection_cleaning_status_producer);
    println!("Cleaning Done: Streamer Disconnected");
}
