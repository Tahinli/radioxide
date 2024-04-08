use std::{mem::MaybeUninit, sync::Arc, time::Duration};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use futures_util::SinkExt;
use ringbuf::{Consumer, HeapRb, Producer, SharedRb};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::broadcast::{channel, Receiver, Sender},
    time::Instant,
};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

use crate::Listener;

const BUFFER_LENGTH: usize = 1000000;
const MAX_TOLERATED_MESSAGE_COUNT:usize = 10;
pub async fn start() {
    let socket = TcpListener::bind("192.168.1.2:2424").await.unwrap();
    println!("Dude Someone Triggered");
    let ring = HeapRb::<f32>::new(BUFFER_LENGTH);
    let (producer, consumer) = ring.split();
    let timer = Instant::now();
    let (message_producer, _) = channel(BUFFER_LENGTH);
    tokio::spawn(record(producer));
    tokio::spawn(parent_stream(timer, message_producer.clone(), consumer));
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
pub async fn parent_stream(
    timer: Instant,
    message_producer: Sender<Message>,
    mut consumer: Consumer<f32, Arc<SharedRb<f32, Vec<MaybeUninit<f32>>>>>,
) {
    loop {
        let mut single_message: Vec<u8> = Vec::new();
        let now = timer.elapsed().as_secs();
        while !consumer.is_empty() && (timer.elapsed().as_secs() + 5) > now {
            match consumer.pop() {
                Some(single_data) => {
                    let ring = HeapRb::<u8>::new(1000000);
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
                None => {}
            }
        }
        match message_producer.send(single_message.into()) {
            Ok(_) => {}
            Err(_) => {}
        }
        println!(
            "Message Len = {} | Receiver Count = {}",
            message_producer.len(),
            message_producer.receiver_count()
        );
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
pub async fn stream(
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

pub async fn record(mut producer: Producer<f32, Arc<SharedRb<f32, Vec<MaybeUninit<f32>>>>>) {
    println!("Hello, world!");
    let host = cpal::default_host();
    let input_device = host.default_input_device().unwrap();

    println!("Input Device: {}", input_device.name().unwrap());

    let config: cpal::StreamConfig = input_device.default_input_config().unwrap().into();

    let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        for &sample in data {
            match producer.push(sample) {
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
