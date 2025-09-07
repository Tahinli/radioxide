use std::{cmp::min, io::Write, sync::Arc, time::Duration};

use brotli::CompressorWriter;
use futures_util::SinkExt;
use ringbuf::HeapRb;
use tokio::{
    sync::{
        broadcast::{channel, Receiver, Sender},
        Mutex,
    },
    task::JoinHandle,
};
use tokio_tungstenite::tungstenite::Message;

use crate::{Config, BUFFER_LENGTH};
const MAX_TOLERATED_MESSAGE_COUNT: usize = 10;

pub async fn connect(
    microphone_stream_receiver: Receiver<f32>,
    audio_stream_receiver: Receiver<f32>,
    streamer_config: Config,
    mut base_to_streaming: Receiver<bool>,
    streaming_to_base: Sender<bool>,
    streaming_to_base_sender_is_finished: Sender<bool>,
    microphone_stream_volume: Arc<Mutex<f32>>,
    audio_stream_volume: Arc<Mutex<f32>>,
) {
    let connect_addr = match streamer_config.tls {
        true => format!("wss://{}", streamer_config.address),
        false => format!("ws://{}", streamer_config.address),
    };

    if let Err(_) = base_to_streaming.try_recv() {
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
                        match streaming_to_base_sender_is_finished.send(true) {
                            Ok(_) => {}
                            Err(err_val) => {
                                eprintln!(
                                    "Error: Communication | Streaming to Base | Send | WSS | Is Finished | {}",
                                    err_val
                                );
                            }
                        }
                        return;
                    }
                }
            }
            false => match tokio_tungstenite::connect_async(connect_addr.clone()).await {
                Ok(ws_stream_connected) => ws_stream = ws_stream_connected.0,
                Err(_) => {
                    match streaming_to_base_sender_is_finished.send(true) {
                        Ok(_) => {}
                        Err(err_val) => {
                            eprintln!(
                                "Error: Communication | Streaming to Base | Send | WS | Is Finished | {}",
                                err_val
                            );
                        }
                    }
                    return;
                }
            },
        }
        let (message_producer, message_consumer) = channel(BUFFER_LENGTH);
        println!("Connected to: {}", connect_addr);
        let (flow_sender, flow_receiver) = channel(BUFFER_LENGTH);
        let mixer_task = tokio::spawn(mixer(
            microphone_stream_receiver,
            audio_stream_receiver,
            microphone_stream_volume,
            audio_stream_volume,
            flow_sender,
            streamer_config.latency,
        ));
        let message_organizer_task = tokio::spawn(message_organizer(
            message_producer,
            flow_receiver,
            streamer_config.quality,
            streamer_config.latency,
        ));
        let stream_task = tokio::spawn(stream(ws_stream, message_consumer));
        let _ = streaming_to_base.send(true);
        tokio::spawn(status_checker(
            message_organizer_task,
            stream_task,
            mixer_task,
            base_to_streaming,
            streaming_to_base,
            streaming_to_base_sender_is_finished,
        ));
    }
}
async fn mixer(
    mut microphone_stream_receiver: Receiver<f32>,
    mut audio_stream_receiver: Receiver<f32>,
    microphone_stream_volume: Arc<Mutex<f32>>,
    audio_stream_volume: Arc<Mutex<f32>>,
    flow_sender: Sender<f32>,
    latency: u16,
) {
    microphone_stream_receiver = microphone_stream_receiver.resubscribe();
    audio_stream_receiver = audio_stream_receiver.resubscribe();
    loop {
        let mut microphone_stream = vec![];
        let mut audio_stream = vec![];

        let mut microphone_stream_iteration = microphone_stream_receiver.len();
        let mut audio_stream_iteration = audio_stream_receiver.len();

        if microphone_stream_iteration > 0 && audio_stream_iteration > 0 {
            let sync_iteration = min(microphone_stream_iteration, audio_stream_iteration);
            microphone_stream_iteration = sync_iteration;
            audio_stream_iteration = sync_iteration;
        }

        while microphone_stream_iteration > 0 {
            microphone_stream_iteration -= 1;
            match microphone_stream_receiver.recv().await {
                Ok(microphone_datum) => {
                    microphone_stream.push(microphone_datum);
                }
                Err(err_val) => {
                    eprintln!(
                        "Error: Communication | Microphone Stream | Recv | {}",
                        err_val
                    );
                }
            }
        }

        while audio_stream_iteration > 0 {
            audio_stream_iteration -= 1;
            match audio_stream_receiver.recv().await {
                Ok(audio_datum) => {
                    audio_stream.push(audio_datum);
                }
                Err(err_val) => {
                    eprintln!("Error: Communication | Audio Stream | Recv | {}", err_val);
                }
            }
        }

        let mut flow = vec![];
        let microphone_volume = *microphone_stream_volume.lock().await;
        let audio_volume = *audio_stream_volume.lock().await;

        for element in microphone_stream {
            if element < 0.01 || element > -0.01 {
                flow.push(element * microphone_volume);
            }
        }

        for (i, element) in audio_stream.iter().enumerate() {
            let audio_volumized = element * audio_volume;
            if flow.len() > i {
                flow[i] = flow[i] + audio_volumized;
            } else {
                flow.push(audio_volumized);
            }
        }

        for i in 0..flow.len() {
            if flow[i] > 1.0 {
                flow[i] = 0.5 * (flow[i] / flow[i].trunc() * 10.0);
            }
        }
        for element in flow {
            match flow_sender.send(element) {
                Ok(_) => {}
                Err(err_val) => {
                    eprintln!("Error: Communication | Flow | Send | {}", err_val);
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(latency.into())).await;
    }
}
async fn message_organizer(
    message_producer: Sender<Message>,
    mut flow_receiver: Receiver<f32>,
    quality: u8,
    latency: u16,
) {
    loop {
        let mut messages: Vec<u8> = Vec::new();
        let mut iteration = flow_receiver.len();
        while iteration > 0 {
            iteration -= 1;
            match flow_receiver.recv().await {
                Ok(single_data) => {
                    let ring = HeapRb::<u8>::new(BUFFER_LENGTH);
                    let (mut producer, mut flow_receiver) = ring.split();
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
                    while !flow_receiver.is_empty() {
                        messages.push(flow_receiver.pop().unwrap());
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
            match message_producer.send(compressed_messages.into()) {
                Ok(_) => {}
                Err(_) => {}
            }
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
            break;
        }
        match ws_stream.send(message).await {
            Ok(_) => {
                if let Err(_) = ws_stream.flush().await {
                    break;
                }
            }
            Err(_) => {
                break;
            }
        }
    }
}

async fn status_checker(
    message_organizer_task: JoinHandle<()>,
    stream_task: JoinHandle<()>,
    mixer_task: JoinHandle<()>,
    mut base_to_streaming: Receiver<bool>,
    streaming_to_base: Sender<bool>,
    streaming_to_base_sender_is_finished: Sender<bool>,
) {
    let mut problem = false;
    loop {
        if let Ok(_) = base_to_streaming.try_recv() {
            println!("Time to Retrieve");
            break;
        }
        if stream_task.is_finished() {
            println!("Warning: Stream Task Finished");
            problem = true;
            break;
        }
        if mixer_task.is_finished() {
            println!("Warning: Mixer Task Finished");
            problem = true;
            break;
        }
        if message_organizer_task.is_finished() {
            println!("Warning: Message Organizer Task Finished");
            problem = true;
            break;
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
    stream_task.abort();
    mixer_task.abort();
    message_organizer_task.abort();
    if problem {
        match streaming_to_base_sender_is_finished.send(true) {
            Ok(_) => println!("Cleaning Done: Streamer Disconnected"),
            Err(err_val) => eprintln!("Error: Cleaning | Is Finished | {}", err_val),
        }
    } else {
        match streaming_to_base.send(true) {
            Ok(_) => println!("Cleaning Done: Streamer Disconnected"),
            Err(err_val) => eprintln!("Error: Cleaning | Is Stopped | {}", err_val),
        }
    }
}
