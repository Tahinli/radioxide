use std::{mem::MaybeUninit, sync::Arc};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{Consumer, HeapRb, Producer, SharedRb};
use tokio::net::{TcpListener, TcpStream};
use futures_util::SinkExt;
use tokio_tungstenite::WebSocketStream;

pub async fn start() {
    let socket = TcpListener::bind("127.0.0.1:2424").await.unwrap();
    while let Ok((tcp_stream, _)) = socket.accept().await {
        println!("Dude Someone Triggered");
        let ring = HeapRb::<f32>::new(1000000);
        let (producer, consumer) = ring.split();
        let ws_stream = tokio_tungstenite::accept_async(tcp_stream).await.unwrap();
        
        tokio::spawn(record(producer));
        std::thread::sleep(std::time::Duration::from_secs(3));
        tokio::spawn(stream(ws_stream, consumer));
    }
}

pub async fn stream(mut ws_stream:WebSocketStream<TcpStream>, mut consumer: Consumer<f32, Arc<SharedRb<f32, Vec<MaybeUninit<f32>>>>>) {
    println!("Waiting");
    loop {
        if !consumer.is_empty() {
            match consumer.pop() {
                Some(data) => {
                    ws_stream.send(data.to_string().into()).await.unwrap();       
                }
                None => {
                    //ws_stream.send(0.0.to_string().into()).await.unwrap();
                }
            }
            ws_stream.flush().await.unwrap();
        }           
    }
}

pub async fn record(mut producer: Producer<f32, Arc<SharedRb<f32, Vec<MaybeUninit<f32>>>>>) {
    println!("Hello, world!");
    let host = cpal::default_host();
    let input_device = host.default_input_device().unwrap();

    println!("Input Device: {}", input_device.name().unwrap());

    let config:cpal::StreamConfig = input_device.default_input_config().unwrap().into();

    let input_data_fn = move |data: &[f32], _:&cpal::InputCallbackInfo| {
        for &sample in data {
            match producer.push(sample) {
                Ok(_) => {},
                Err(_) => {},
            }
            println!("{}", sample);
        }
    };
    

    let input_stream = input_device.build_input_stream(&config, input_data_fn, err_fn, None).unwrap();
    

    println!("STREAMIN");
    input_stream.play().unwrap();
    std::thread::sleep(std::time::Duration::from_secs(100));
    println!("DONE I HOPE");
}

fn err_fn(err: cpal::StreamError) {
    eprintln!("Something Happened: {}", err);
}
