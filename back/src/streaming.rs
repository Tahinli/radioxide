use std::{
    fs::File,
    io::{self, BufReader},
    sync::Arc,
    time::Duration,
};

use futures_util::{SinkExt, StreamExt};
use rustls_pemfile::{certs, pkcs8_private_keys};

use tokio::{
    net::TcpListener,
    sync::broadcast::{channel, Receiver, Sender},
    task::JoinHandle,
    time::Instant,
};
use tokio_rustls::{
    rustls::pki_types::{CertificateDer, PrivateKeyDer},
    TlsAcceptor,
};
use tokio_tungstenite::tungstenite::{Error, Message};

use crate::{Config, Listener, Streamer};

const BUFFER_LENGTH: usize = 1000000;
const MAX_TOLERATED_MESSAGE_COUNT: usize = 10;
pub async fn start(relay_configs: Config) {
    //need to move them for multi streamer
    let listener_socket = TcpListener::bind(relay_configs.listener_address)
        .await
        .unwrap();
    let (record_producer, record_consumer) = channel(BUFFER_LENGTH);
    let streamer_socket = TcpListener::bind(relay_configs.streamer_address)
        .await
        .unwrap();
    let timer = Instant::now();

    let fullchain: io::Result<Vec<CertificateDer<'static>>> = certs(&mut BufReader::new(
        File::open("certificates/fullchain.pem").unwrap(),
    ))
    .collect();
    let fullchain = fullchain.unwrap();
    let privkey: io::Result<PrivateKeyDer<'static>> = pkcs8_private_keys(&mut BufReader::new(
        File::open("certificates/privkey.pem").unwrap(),
    ))
    .next()
    .unwrap()
    .map(Into::into);
    let privkey = privkey.unwrap();

    let server_tls_config = tokio_rustls::rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(fullchain, privkey)
        .unwrap();
    let acceptor = TlsAcceptor::from(Arc::new(server_tls_config));

    let (streamer_alive_producer, streamer_alive_receiver) = tokio::sync::oneshot::channel();
    let message_organizer_task: Option<JoinHandle<()>>;
    let buffer_layer_task: Option<JoinHandle<()>>;
    let (listener_stream_tasks_producer, listener_stream_tasks_receiver) =
        tokio::sync::mpsc::channel(BUFFER_LENGTH);
    let mut new_streamer = Streamer {
        ip: "127.0.0.1".to_string().parse().unwrap(),
        port: 0000,
    };
    loop {
        match streamer_socket.accept().await {
            Ok((streamer_tcp, streamer_info)) => {
                new_streamer.ip = streamer_info.ip();
                new_streamer.port = streamer_info.port();
                println!(
                    "New Streamer: {:#?} | {:#?}",
                    streamer_info,
                    timer.elapsed()
                );
                if relay_configs.tls {
                    let streamer_tcp_tls = acceptor.accept(streamer_tcp).await.unwrap();
                    match tokio_tungstenite::accept_async(streamer_tcp_tls).await {
                        Ok(ws_stream) => {
                            tokio::spawn(streamer_stream(
                                new_streamer.clone(),
                                record_producer,
                                ws_stream,
                                timer,
                                streamer_alive_producer,
                            ));
                            break;
                        }
                        Err(err_val) => eprintln!("Error: TCP to WS Transform | {}", err_val),
                    }
                } else {
                    match tokio_tungstenite::accept_async(streamer_tcp).await {
                        Ok(ws_stream) => {
                            tokio::spawn(streamer_stream(
                                new_streamer.clone(),
                                record_producer,
                                ws_stream,
                                timer,
                                streamer_alive_producer,
                            ));
                            break;
                        }
                        Err(err_val) => eprintln!("Error: TCP to WS Transform | {}", err_val),
                    }
                }
            }
            Err(err_val) => eprintln!("Error: TCP Accept Connection | {}", err_val),
        }
    }
    let (message_producer, message_consumer) = channel(BUFFER_LENGTH);
    let (buffered_producer, _) = channel(BUFFER_LENGTH);
    message_organizer_task = tokio::spawn(message_organizer(
        message_producer.clone(),
        record_consumer,
        relay_configs.latency,
    ))
    .into();
    buffer_layer_task = tokio::spawn(buffer_layer(
        message_consumer,
        buffered_producer.clone(),
        relay_configs.latency,
    ))
    .into();
    tokio::spawn(status_checker(
        buffered_producer.clone(),
        timer,
        new_streamer,
        streamer_alive_receiver,
        message_organizer_task,
        buffer_layer_task,
        listener_stream_tasks_receiver,
    ));
    while let Ok((tcp_stream, listener_info)) = listener_socket.accept().await {
        let new_listener = Listener {
            ip: listener_info.ip(),
            port: listener_info.port(),
        };
        if relay_configs.tls {
            let streamer_tcp_tls = acceptor.accept(tcp_stream).await.unwrap();
            let wss_stream = tokio_tungstenite::accept_async(streamer_tcp_tls)
                .await
                .unwrap();
            let listener_stream_task = tokio::spawn(stream(
                new_listener,
                wss_stream,
                buffered_producer.subscribe(),
            ));
            let _ = listener_stream_tasks_producer
                .send(listener_stream_task)
                .await;
        } else {
            let ws_stream = tokio_tungstenite::accept_async(tcp_stream).await.unwrap();
            let listener_stream_task = tokio::spawn(stream(
                new_listener,
                ws_stream,
                buffered_producer.subscribe(),
            ));
            let _ = listener_stream_tasks_producer
                .send(listener_stream_task)
                .await;
        }
        println!("New Listener: {} | {:#?}", listener_info, timer.elapsed());
    }
}
async fn status_checker(
    buffered_producer: Sender<Message>,
    timer: Instant,
    streamer: Streamer,
    mut streamer_alive_receiver: tokio::sync::oneshot::Receiver<bool>,
    message_organizer_task: Option<JoinHandle<()>>,
    buffer_layer_task: Option<JoinHandle<()>>,
    mut listener_stream_tasks_receiver: tokio::sync::mpsc::Receiver<JoinHandle<()>>,
) {
    let mut listener_counter = buffered_producer.receiver_count();
    let mut bottleneck_flag = false;
    //let mut buffer_len = buffered_producer.len();
    loop {
        tokio::time::sleep(Duration::from_secs(3)).await;
        match streamer_alive_receiver.try_recv() {
            Ok(_) => {
                println!(
                    "Cleaning: Streamer Disconnected | {}",
                    format!("{}:{}", streamer.ip, streamer.port)
                );
                let cleaning_timer = Instant::now();
                message_organizer_task.as_ref().unwrap().abort();
                buffer_layer_task.as_ref().unwrap().abort();
                let mut listener_task_counter = 0;
                while listener_stream_tasks_receiver.len() > 0 {
                    match listener_stream_tasks_receiver.recv().await {
                        Some(listener_stream_task) => {
                            listener_stream_task.abort();
                            listener_task_counter += 1;
                        }
                        None => {}
                    }
                }
                println!(
                    "Cleaning Done: Streamer Disconnected | {} | Disconnected Listener(s) = {} | {:#?}",
                    format!("{}:{}", streamer.ip, streamer.port),
                    listener_task_counter,
                    cleaning_timer.elapsed()
                );
                return;
            }
            Err(_) => {}
        }

        if buffered_producer.receiver_count() != 0 {
            if buffered_producer.len() > 2 {
                bottleneck_flag = true;
                println!(
                    "Bottleneck: {} | {:#?}",
                    buffered_producer.len(),
                    timer.elapsed()
                );
            }
            if bottleneck_flag && buffered_producer.len() < 2 {
                bottleneck_flag = false;
                println!("Flawless Again");
            }
            if listener_counter != buffered_producer.receiver_count() {
                listener_counter = buffered_producer.receiver_count();
                println!("Listener(s): {}", listener_counter);
            }
        }
    }
}
async fn buffer_layer(
    mut message_consumer: Receiver<Message>,
    buffered_producer: Sender<Message>,
    delay: u16,
) {
    loop {
        tokio::time::sleep(Duration::from_millis(delay.into())).await;
        while message_consumer.len() > 0 {
            match message_consumer.recv().await {
                Ok(message) => match buffered_producer.send(message) {
                    Ok(_) => {}
                    Err(_) => {}
                },
                Err(_) => {}
            }
        }
    }
}
async fn streamer_stream<
    T: futures_util::Stream<Item = Result<Message, Error>> + std::marker::Unpin,
>(
    streamer: Streamer,
    record_producer: Sender<Message>,
    mut ws_stream: T,
    timer: Instant,
    streamer_alive_producer: tokio::sync::oneshot::Sender<bool>,
) {
    loop {
        match ws_stream.next().await {
            Some(message_with_question) => {
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
            None => {
                println!(
                    "Streamer Disconnected: {} | {:#?}",
                    format!("{}:{}", streamer.ip, streamer.port),
                    timer.elapsed()
                );
                streamer_alive_producer.send(false).unwrap();
                return;
            }
        }
    }
}

async fn message_organizer(
    message_producer: Sender<Message>,
    mut record_consumer: Receiver<Message>,
    delay: u16,
) {
    loop {
        match record_consumer.recv().await {
            Ok(single_message) => match message_producer.send(single_message) {
                Ok(_) => {}
                Err(_) => {}
            },
            Err(_) => {}
        }
        tokio::time::sleep(Duration::from_millis(delay.into())).await;
    }
}
async fn stream<T: futures_util::Sink<Message> + std::marker::Unpin>(
    listener: Listener,
    mut ws_stream: T,
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
