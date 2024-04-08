use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use futures_util::SinkExt;
use ringbuf::HeapRb;
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

    let (message_producer, _) = channel(BUFFER_LENGTH);
    tokio::spawn(record(record_producer));
    tokio::spawn(message_organizer(message_producer.clone(), record_consumer));
    while let Ok((tcp_stream, info)) = socket.accept().await {
        let ws_stream = tokio_tungstenite::accept_async(tcp_stream).await.unwrap();
        println!("New Connection: {}", info);
        let new_listener = Listener {
            ip: info.ip(),
            port: info.port(),
        };
        tokio::spawn(stream(
            new_listener,
            ws_stream,
            message_producer.subscribe(),
        ));
    }
}
async fn message_organizer(message_producer: Sender<Message>, mut consumer: Receiver<f32>) {
    loop {
        let mut single_message: Vec<u8> = Vec::new();
        let mut iteration = consumer.len();
        while iteration > 0 {
            iteration -= 1;
            match consumer.recv().await {
                Ok(single_data) => {
                    let ring = HeapRb::<u8>::new(BUFFER_LENGTH);
                    let (mut producer, mut consumer) = ring.split();
                    let single_data_packet = single_data.to_string().as_bytes().to_vec();
                    let terminator = "#".as_bytes().to_vec();

                    for element in single_data_packet {
                        producer.push(element).unwrap();
                    }
                    for element in terminator {
                        producer.push(element).unwrap();
                    }
                    while !consumer.is_empty() {
                        single_message.push(consumer.pop().unwrap());
                    }
                }
                Err(_) => {}
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
        if !single_message.is_empty() {
            match message_producer.send(single_message.into()) {
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

async fn record(producer: Sender<f32>) {
    println!("Hello, world!");
    let host = cpal::default_host();
    let input_device = host.default_input_device().unwrap();

    println!("Input Device: {}", input_device.name().unwrap());

    let config: cpal::StreamConfig = input_device.default_input_config().unwrap().into();

    let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        for &sample in data {
            match producer.send(sample) {
                Ok(_) => {}
                Err(_) => {}
            }
            //println!("{}", sample);
        }
    };

    let input_stream = input_device
        .build_input_stream(&config, input_data_fn, err_fn, None)
        .unwrap();

    println!("STREAMIN");
    input_stream.play().unwrap();
    //oneshot ile durdurabiliriz sanırım
    std::thread::sleep(std::time::Duration::from_secs(10000000));
    println!("DONE I HOPE");
}

fn err_fn(err: cpal::StreamError) {
    eprintln!("Something Happened: {}", err);
}
