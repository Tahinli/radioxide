
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::broadcast::{channel, Receiver, Sender},
};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

use crate::Listener;

const BUFFER_LENGTH: usize = 1000000;
const MAX_TOLERATED_MESSAGE_COUNT: usize = 10;
pub async fn start() {
    let socket = TcpListener::bind("192.168.1.2:2424").await.unwrap();
    println!("Dude Someone Triggered");
    let (record_producer, record_consumer) = channel(BUFFER_LENGTH);
    let streamer_socket = TcpListener::bind("192.168.1.2:2525").await.unwrap();
    if let Ok((streamer_tcp, streamer_info)) = streamer_socket.accept().await {
        let ws_stream = tokio_tungstenite::accept_async(streamer_tcp).await.unwrap();
        println!("New Streamer: {:#?}", streamer_info);
        tokio::spawn(streamer_stream(record_producer, ws_stream));
    }

    let (message_producer, _) = channel(BUFFER_LENGTH);
    tokio::spawn(message_organizer(message_producer.clone(), record_consumer));
    while let Ok((tcp_stream, listener_info)) = socket.accept().await {
        let ws_stream = tokio_tungstenite::accept_async(tcp_stream).await.unwrap();
        println!("New Listener: {}", listener_info);
        let new_listener = Listener {
            ip: listener_info.ip(),
            port: listener_info.port(),
        };
        tokio::spawn(stream(
            new_listener,
            ws_stream,
            message_producer.subscribe(),
        ));
    }
}

async fn streamer_stream(record_producer:Sender<Message>, mut ws_stream: WebSocketStream<TcpStream>) {
    while let Some(message_with_question) = ws_stream.next().await {
        match message_with_question {
            Ok(message) => {
                match record_producer.send(message) {
                    Ok(_) => {

                    }
                    Err(_) => {

                    }
                }
            }
            Err(_) => {

            }
        }
    }
}

async fn message_organizer(message_producer: Sender<Message>, mut consumer: Receiver<Message>) {
    loop {
        let mut messages:Vec<u8> = vec![];
        let mut iteration = consumer.len();
        while iteration > 0 {
            iteration -= 1;
            match consumer.recv().await {
                Ok(single_message) => {
                    let single_message_packet = single_message.to_string().as_bytes().to_vec();
                    for element in single_message_packet {
                        messages.push(element);
                    }
                }
                Err(_) => {

                }
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
        if !messages.is_empty() {
            match message_producer.send(messages.into()) {
                Ok(_) => {}
                Err(_) => {}
            }
            println!(
                "Message Counter = {} | Receiver Count = {}",
                message_producer.len(),
                message_producer.receiver_count()
            );
        }
    }
}
async fn stream(
    listener: Listener,
    mut ws_stream: WebSocketStream<TcpStream>,
    mut message_consumer: Receiver<Message>,
) {
    while let Ok(message) = message_consumer.recv().await {
        if message_consumer.len() > MAX_TOLERATED_MESSAGE_COUNT {
            println!(
                "{} Forced to Disconnect | Reason -> Slow Consumer",
                format!("{}:{}", listener.ip, listener.port)
            );
            break;
        }
        match ws_stream.send(message).await {
            Ok(_) => {
                if let Err(_) = ws_stream.flush().await {
                    println!(
                        "{} is Disconnected",
                        format!("{}:{}", listener.ip, listener.port)
                    );
                    break;
                }
            }
            Err(_) => {
                println!(
                    "{} is Disconnected",
                    format!("{}:{}", listener.ip, listener.port)
                );
                break;
            }
        }
    }
}

