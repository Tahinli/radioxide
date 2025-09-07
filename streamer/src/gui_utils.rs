use tokio::sync::broadcast::{Receiver, Sender};

use crate::{gui::State, recording, streaming, Config};

pub async fn connect(
    sound_stream_receiver: Receiver<f32>,
    streamer_config: Config,
    streaming_to_base: Sender<bool>,
    base_to_streaming: Receiver<bool>,
) -> State {
    let mut streaming_to_base_receiver = streaming_to_base.subscribe();
    tokio::spawn(streaming::connect(
        sound_stream_receiver,
        streamer_config,
        base_to_streaming,
        streaming_to_base.clone(),
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
    mut streaming_to_base: Receiver<bool>,
    base_to_streaming: Sender<bool>,
) -> State {
    let _ = base_to_streaming.send(false);
    match streaming_to_base.recv().await {
        Ok(_) => State::Disconnected,
        Err(err_val) => {
            eprintln!("Error: Communication | {}", err_val);
            State::Connected
        }
    }
}

pub async fn start_recording(
    sound_stream_sender: Sender<f32>,
    recording_to_base: Sender<bool>,
    base_to_recording: Receiver<bool>,
) -> State {
    let mut recording_to_base_receiver = recording_to_base.subscribe();
    tokio::spawn(recording::record(
        sound_stream_sender.clone(),
        recording_to_base.clone(),
        base_to_recording,
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
    mut recording_to_base: Receiver<bool>,
    base_to_recording: Sender<bool>,
) -> State {
    let _ = base_to_recording.send(false);
    match recording_to_base.recv().await {
        Ok(_) => State::StopRecording,
        Err(err_val) => {
            eprintln!("Error: Communication | {}", err_val);
            State::Recording
        }
    }
}

pub async fn start_playing_audio(
    playing_audio_to_base: Sender<bool>,
    base_to_playing_audio: Receiver<bool>,
) -> State {
    //tokio::spawn(future);
    let mut playing_audio_to_base_receiver = playing_audio_to_base.subscribe();
    match playing_audio_to_base_receiver.recv().await {
        Ok(_) => State::PlayingAudio,
        Err(err_val) => {
            eprint!("Error: Communication | Playing | {}", err_val);
            State::StopAudio
        }
    }
}

pub async fn stop_playing_audio(
    mut audio_to_base: Receiver<bool>,
    base_to_audio: Sender<bool>,
) -> State {
    let _ = base_to_audio.send(false);
    match audio_to_base.recv().await {
        Ok(_) => State::StopRecording,
        Err(err_val) => {
            eprintln!("Error: Communication | {}", err_val);
            State::Recording
        }
    }
}
