use iced::{
    widget::{button, column, Column},
    Command,
};
use tokio::sync::broadcast::{channel, Sender};

use crate::{recording, streaming, utils::get_config, Config, BUFFER_LENGTH};

#[derive(Debug, Clone)]
pub enum Message {
    StartStreaming,
    StopStreaming,
    ConfigLoad(Config),
}

#[derive(Debug)]
pub struct Streamer {
    config: Option<Config>,
    sound_stream_producer: Sender<f32>,
    stop_connection_producer: Sender<bool>,
    stop_recording_producer: Sender<bool>,
}
impl Default for Streamer {
    fn default() -> Self {
        Self::new()
    }
}

impl Streamer {
    fn new() -> Self {
        Self {
            config: None,
            sound_stream_producer: channel(BUFFER_LENGTH).0,
            stop_connection_producer: channel(BUFFER_LENGTH).0,
            stop_recording_producer: channel(BUFFER_LENGTH).0,
        }
    }
    pub fn update(&mut self, message: Message) {
        match message {
            Message::StartStreaming => {
                println!("Start Stream");
                tokio::spawn(streaming::connect(
                    self.sound_stream_producer.subscribe(),
                    self.config.clone().unwrap(),
                    self.stop_connection_producer.subscribe(),
                ));
                tokio::spawn(recording::record(
                    self.sound_stream_producer.clone(),
                    self.stop_recording_producer.subscribe(),
                ));
            }
            Message::StopStreaming => {
                println!("Stop Stream");
                self.stop_connection_producer.send(true).unwrap();
                self.stop_recording_producer.send(true).unwrap();
            }
            Message::ConfigLoad(config) => {
                self.config = Some(config);
            }
        }
    }
    pub fn view(&self) -> Column<Message> {
        column![
            button("Start Streaming").on_press(Message::StartStreaming),
            button("Stop Streaming").on_press(Message::StopStreaming),
        ]
    }
    pub fn load_config() -> Command<Message> {
        Command::perform(get_config(), Message::ConfigLoad)
    }
}
