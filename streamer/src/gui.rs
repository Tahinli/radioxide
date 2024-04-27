use iced::{widget::{button, column, Column}, Command};
use tokio::sync::broadcast::{channel, Sender};

use crate::{recording, streaming, utils::get_config, Config, BUFFER_LENGTH};

#[derive(Debug, Clone)]
pub enum Message {
    StartStreaming,
    ConfigLoad(Config),
}

#[derive(Debug)]
pub struct Streamer {
    config: Option<Config>,
    sound_stream_producer:Sender<f32>,
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
        }
    }
    pub fn update(&mut self, message:Message) {
        match message {
            Message::StartStreaming => {
                tokio::spawn(streaming::connect(self.sound_stream_producer.subscribe(), self.config.clone().unwrap()));
                tokio::spawn(recording::record(self.sound_stream_producer.clone()));
            }
            Message::ConfigLoad(config) => {
                self.config = Some(config);
            }
        }
    }
    pub fn view(&self) -> Column<Message> {
        column![
            button("Start Streaming").on_press(Message::StartStreaming)
        ]
    }
    pub fn load_config() -> Command<Message> {
        Command::perform(get_config(), Message::ConfigLoad)
    }
}



