use std::{mem::MaybeUninit, sync::Arc, time::Duration};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{Consumer, HeapRb, Producer, SharedRb};
use tokio::{net::{TcpListener, TcpStream}, time::Instant};
use futures_util::SinkExt;
use tokio_tungstenite::WebSocketStream;


pub async fn start() {
    let socket = TcpListener::bind("192.168.1.2:2424").await.unwrap();
    while let Ok((tcp_stream, _)) = socket.accept().await {
        println!("Dude Someone Triggered");
        let ring = HeapRb::<f32>::new(1000000);
        let (producer, consumer) = ring.split();
        let ws_stream = tokio_tungstenite::accept_async(tcp_stream).await.unwrap();

        let timer = Instant::now();


        tokio::spawn(record(producer));
        tokio::spawn(stream(timer, ws_stream, consumer));
    }
}

pub async fn stream(timer:Instant, mut ws_stream:WebSocketStream<TcpStream>, mut consumer: Consumer<f32, Arc<SharedRb<f32, Vec<MaybeUninit<f32>>>>>) {
    println!("Waiting");
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let mut data:Vec<u8> = Vec::new();
        let now = timer.elapsed().as_secs();
        while !consumer.is_empty() && (timer.elapsed().as_secs()+2) > now{
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
                        data.push(consumer.pop().unwrap());
                    }
                }
                None => {

                }
            }
        }
        ws_stream.send(data.into()).await.unwrap();
        ws_stream.flush().await.unwrap();
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
