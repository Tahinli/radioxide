use std::{fs::File, time::Duration};

use tokio::sync::broadcast::{Receiver, Sender};

use crate::{
    gui::{Player, State},
    playing, recording, streaming, Config,
};

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
            eprintln!("Error: Communication | Streaming to Base | {}", err_val);
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
            eprintln!("Error: Communication | Streaming to Base | {}", err_val);
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
            eprintln!("Error: Communication | Recording to Base | {}", err_val);
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
            eprintln!("Error: Communication | Recording to Base | {}", err_val);
            State::Recording
        }
    }
}

pub async fn start_playing(
    audio_stream_sender: Sender<f32>,
    decoded_to_playing_sender: Sender<f32>,
    file: File,
    playing_to_base_sender: Sender<Player>,
    base_to_playing_receiver: Receiver<Player>,
) -> State {
    let mut playing_to_base_receiver = playing_to_base_sender.subscribe();
    tokio::spawn(playing::play(
        audio_stream_sender,
        file,
        decoded_to_playing_sender,
        playing_to_base_sender,
        base_to_playing_receiver,
    ));
    let answer = playing_to_base_receiver.recv().await;
    drop(playing_to_base_receiver);
    match answer {
        Ok(state) => match state {
            Player::Play => State::PlayingAudio,
            Player::Pause => State::PausedAudio,
            Player::Stop => State::StopAudio,
        },
        Err(err_val) => {
            eprint!(
                "Error: Communication | Playing to Base | Recv | Start | {}",
                err_val
            );
            State::StopAudio
        }
    }
}

pub async fn stop_playing(
    mut playing_to_base_receiver: Receiver<Player>,
    base_to_playing_sender: Sender<Player>,
) -> State {
    match base_to_playing_sender.send(Player::Stop) {
        Ok(_) => {}
        Err(err_val) => {
            eprintln!(
                "Error: Communication | Base to Playing | Stop | Send | {}",
                err_val
            );
        }
    }
    drop(base_to_playing_sender);
    let answer = playing_to_base_receiver.recv().await;
    drop(playing_to_base_receiver);
    match answer {
        Ok(state) => match state {
            Player::Play => State::PlayingAudio,
            Player::Pause => State::PausedAudio,
            Player::Stop => State::StopAudio,
        },
        Err(err_val) => {
            eprintln!(
                "Error: Communication | Playing to Base | Recv | Stop | {}",
                err_val
            );
            State::PlayingAudio
        }
    }
}

pub async fn is_playing_finished(
    mut playing_to_base_receiver_is_audio_finished: Receiver<Player>,
    mut playing_to_base_receiver_is_audio_stopped: Receiver<Player>,
    base_to_playing_sender: Sender<Player>,
    decoded_to_playing_sender: Sender<f32>,
) -> State {
    tokio::select! {
        is_audio_finished = tokio::spawn(async move {
            match playing_to_base_receiver_is_audio_finished.recv().await {
                Ok(state) => match state {
                    Player::Play => {
                        while !decoded_to_playing_sender.is_empty() {
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                        stop_playing(playing_to_base_receiver_is_audio_finished, base_to_playing_sender).await
                    }
                    Player::Pause => State::PlayingAudio,
                    Player::Stop => State::StopAudio,
                },
                Err(err_val) => {
                    eprintln!("Error: Communication | Playing to Base | Recv | Is Finish | {}", err_val);
                    State::PlayingAudio
                }
            }
        }) => is_audio_finished.unwrap(),
        is_audio_stopped = tokio::spawn(async move {
            loop {
                match playing_to_base_receiver_is_audio_stopped.recv().await {
                Ok(state) => if let Player::Stop = state {
                    return State::StopAudio;
                },
                Err(err_val) => {
                    eprintln!(
                        "Error: Communication | Playing to Base | Recv | Is Stop | {}",
                        err_val
                    );
                    return State::PlayingAudio;
                }
            }
        }
        })
            =>is_audio_stopped.unwrap(),
    }
}

pub async fn pause_playing(
    mut playing_to_base_receiver: Receiver<Player>,
    base_to_playing_sender: Sender<Player>,
) -> State {
    match base_to_playing_sender.send(Player::Pause) {
        Ok(_) => {}
        Err(err_val) => {
            eprintln!(
                "Error: Communication | Base to Playing | Pause | Send | {}",
                err_val
            );
        }
    }
    drop(base_to_playing_sender);
    let answer = playing_to_base_receiver.recv().await;
    drop(playing_to_base_receiver);
    match answer {
        Ok(state) => match state {
            Player::Play => State::PlayingAudio,
            Player::Pause => State::PausedAudio,
            Player::Stop => State::StopAudio,
        },
        Err(err_val) => {
            eprintln!(
                "Error: Communication | Playing to Base | Recv | Pause | {}",
                err_val
            );
            State::PlayingAudio
        }
    }
}

pub async fn continue_playing(
    mut playing_to_base_receiver: Receiver<Player>,
    base_to_playing_sender: Sender<Player>,
) -> State {
    match base_to_playing_sender.send(Player::Play) {
        Ok(_) => {}
        Err(err_val) => {
            eprintln!(
                "Error: Communication | Base to Playing | Send | Continue | {}",
                err_val
            );
        }
    }
    drop(base_to_playing_sender);
    let answer = playing_to_base_receiver.recv().await;
    drop(playing_to_base_receiver);
    match answer {
        Ok(state) => match state {
            Player::Play => State::PlayingAudio,
            Player::Pause => State::PausedAudio,
            Player::Stop => State::StopAudio,
        },
        Err(err_val) => {
            eprintln!(
                "Error: Communication | Playing to Base | Continue | {}",
                err_val
            );
            State::PausedAudio
        }
    }
}
