use tokio::sync::broadcast::{Receiver, Sender};

use crate::{gui::State, playing, recording, streaming, Config};

pub async fn connect(
    sound_stream_receiver: Receiver<f32>,
    streamer_config: Config,
    streaming_to_base_sender: Sender<bool>,
    base_to_streaming_receiver: Receiver<bool>,
) -> State {
    let mut streaming_to_base_receiver = streaming_to_base_sender.subscribe();
    tokio::spawn(streaming::connect(
        sound_stream_receiver,
        streamer_config,
        base_to_streaming_receiver,
        streaming_to_base_sender.clone(),
    ));
    match streaming_to_base_receiver.recv().await {
        Ok(_) => State::Connected,
        Err(err_val) => {
            eprintln!("Error: Communication | {}", err_val);
            State::Disconnected
        }
    }
}

pub async fn disconnect(
    mut streaming_to_base_receiver: Receiver<bool>,
    base_to_streaming_sender: Sender<bool>,
) -> State {
    let _ = base_to_streaming_sender.send(false);
    match streaming_to_base_receiver.recv().await {
        Ok(_) => State::Disconnected,
        Err(err_val) => {
            eprintln!("Error: Communication | {}", err_val);
            State::Connected
        }
    }
}

pub async fn start_recording(
    microphone_stream_sender: Sender<f32>,
    recording_to_base_sender: Sender<bool>,
    base_to_recording_receiver: Receiver<bool>,
) -> State {
    let mut recording_to_base_receiver = recording_to_base_sender.subscribe();
    tokio::spawn(recording::record(
        microphone_stream_sender.clone(),
        recording_to_base_sender.clone(),
        base_to_recording_receiver,
    ));

    match recording_to_base_receiver.recv().await {
        Ok(_) => State::Recording,
        Err(err_val) => {
            eprintln!("Error: Communication | Streaming | {}", err_val);
            State::StopRecording
        }
    }
}

pub async fn stop_recording(
    mut recording_to_base_receiver: Receiver<bool>,
    base_to_recording_sender: Sender<bool>,
) -> State {
    let _ = base_to_recording_sender.send(false);
    match recording_to_base_receiver.recv().await {
        Ok(_) => State::StopRecording,
        Err(err_val) => {
            eprintln!("Error: Communication | {}", err_val);
            State::Recording
        }
    }
}

pub async fn start_playing(
    audio_stream_sender: Sender<f32>,
    playing_to_base_sender: Sender<bool>,
    base_to_playing_receiver: Receiver<bool>,
) -> State {
    let mut playing_to_base_receiver = playing_to_base_sender.subscribe();
    tokio::spawn(playing::play(
        audio_stream_sender,
        playing_to_base_sender,
        base_to_playing_receiver,
    ));
    match playing_to_base_receiver.recv().await {
        Ok(_) => State::PlayingAudio,
        Err(err_val) => {
            eprint!("Error: Communication | Playing | {}", err_val);
            State::StopAudio
        }
    }
}

pub async fn stop_playing(
    mut playing_to_base_receiver: Receiver<bool>,
    base_to_playing_sender: Sender<bool>,
) -> State {
    let _ = base_to_playing_sender.send(false);
    match playing_to_base_receiver.recv().await {
        Ok(_) => State::StopAudio,
        Err(err_val) => {
            eprintln!("Error: Communication | {}", err_val);
            State::PlayingAudio
        }
    }
}
