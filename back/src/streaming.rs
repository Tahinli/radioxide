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
    let (record_producer, record_consumer) = channel(BUFFER_LENGTH);
    let streamer_socket = TcpListener::bind("192.168.1.2:2525").await.unwrap();
    if let Ok((streamer_tcp, streamer_info)) = streamer_socket.accept().await {
        let ws_stream = tokio_tungstenite::accept_async(streamer_tcp).await.unwrap();
        println!("New Streamer: {:#?}", streamer_info);
        tokio::spawn(streamer_stream(record_producer, ws_stream));
    }

    let (message_producer, message_consumer) = channel(BUFFER_LENGTH);
    let (buffered_producer, _) = channel(BUFFER_LENGTH);
    tokio::spawn(message_organizer(message_producer.clone(), record_consumer));
    tokio::spawn(buffer_layer(message_consumer, buffered_producer.clone()));
    tokio::spawn(status_checker(buffered_producer.clone()));
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
            buffered_producer.subscribe(),
        ));
    }
}
async fn status_checker(buffered_producer: Sender<Message>) {
    let mut listener_counter = buffered_producer.receiver_count();
    //let mut buffer_len = buffered_producer.len();
    loop {
        tokio::time::sleep(Duration::from_secs(3)).await;
        if buffered_producer.receiver_count() != 0 {
            if buffered_producer.len() > 2 {
                println!("Bottleneck: {}", buffered_producer.len());
            }
            if listener_counter != buffered_producer.receiver_count() {
                listener_counter = buffered_producer.receiver_count();
                println!("Listener(s): {}", listener_counter);
            }    
        }
        
    }
}
async fn buffer_layer(mut message_consumer: Receiver<Message>, buffered_producer: Sender<Message>) {
    loop {
        tokio::time::sleep(Duration::from_millis(50)).await;
        while message_consumer.len() > 0 {
            match message_consumer.recv().await {
                Ok(message) => match buffered_producer.send(message) {
                    Ok(_) => {
                        
                    }
                    Err(_) => {}
                },
                Err(_) => {}
            }
        }
    }
}
async fn streamer_stream(
    record_producer: Sender<Message>,
    mut ws_stream: WebSocketStream<TcpStream>,
) {
    while let Some(message_with_question) = ws_stream.next().await {
        match message_with_question {
            Ok(message) => {
                //println!("{}", message.len());
                match record_producer.send(message) {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
            Err(_) => {}
        }
    }
}

async fn message_organizer(
    message_producer: Sender<Message>,
    mut record_consumer: Receiver<Message>,
) {
    loop {
        match record_consumer.recv().await {
            Ok(single_message) => match message_producer.send(single_message) {
                Ok(_) => {}
                Err(_) => {}
            },
            Err(_) => {}
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
async fn stream(
    listener: Listener,
    mut ws_stream: WebSocketStream<TcpStream>,
    mut buffered_consumer: Receiver<Message>,
) {
    while let Ok(message) = buffered_consumer.recv().await {
        if buffered_consumer.len() > MAX_TOLERATED_MESSAGE_COUNT {
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
