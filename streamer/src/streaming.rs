use std::{io::Write, time::Duration};

use brotli::CompressorWriter;
use futures_util::SinkExt;
use ringbuf::HeapRb;
use tokio::sync::broadcast::{channel, Receiver, Sender};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

use crate::BUFFER_LENGTH;
const MAX_TOLERATED_MESSAGE_COUNT: usize = 10;

pub async fn start(sound_stream_consumer: Receiver<f32>) {
    let connect_addr = "ws://192.168.1.2:2525";
    let ws_stream;
    match tokio_tungstenite::connect_async(connect_addr).await {
        Ok(ws_stream_connected) => ws_stream = ws_stream_connected.0,
        Err(_) => {
            return;
        }
    }
    let (message_producer, message_consumer) = channel(BUFFER_LENGTH);
    println!("Connected to: {}", connect_addr);
    tokio::spawn(message_organizer(message_producer, sound_stream_consumer));
    tokio::spawn(stream(ws_stream, message_consumer));
}

async fn message_organizer(message_producer: Sender<Message>, mut consumer: Receiver<f32>) {
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
                    charred.truncate(4);
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
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn stream(
    mut ws_stream: WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
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
