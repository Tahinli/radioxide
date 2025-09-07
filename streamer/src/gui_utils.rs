use tokio::sync::broadcast::{Receiver, Sender};

use crate::gui::State;

pub async fn disconnect(mut streaming_to_base:Receiver<bool>, base_to_streaming:Sender<bool>) -> State {
    let _ = base_to_streaming.send(false);
    match streaming_to_base.recv().await {
        Ok(_) => State::Disconnected,
        Err(err_val) => {
            eprintln!("Error: Communication | {}", err_val);
            State::Connected
        }
    }

}

pub async fn stop_recording(mut recording_to_base:Receiver<bool>, base_to_recording:Sender<bool>) -> State {
    let _ = base_to_recording.send(false);
    match recording_to_base.recv().await {
        Ok(_) => State::StopRecording,
        Err(err_val) => {
            eprintln!("Error: Communication | {}", err_val);
            State::Recording
        }
    }

}

pub async fn stop_playing_audio(mut audio_to_base:Receiver<bool>, base_to_audio:Sender<bool>) -> State {
    let _ = base_to_audio.send(false);
    match audio_to_base.recv().await {
        Ok(_) => State::StopRecording,
        Err(err_val) => {
            eprintln!("Error: Communication | {}", err_val);
            State::Recording
        }
    }

}