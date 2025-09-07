use iced::{
    widget::{button, column, container, Container},
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
    connection_cleaning_status_producer: Sender<bool>,
    are_we_streaming: bool,
    are_we_recovering: bool,
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
            connection_cleaning_status_producer: channel(BUFFER_LENGTH).0,
            are_we_streaming: false,
            are_we_recovering: false,
        }
    }
    pub fn update(&mut self, message: Message) {
        match message {
            Message::StartStreaming => {
                if !self.are_we_recovering && !self.are_we_streaming {
                    println!("Start Stream");
                    self.are_we_recovering = true;
                    self.are_we_streaming = true;
                    tokio::spawn(streaming::connect(
                        self.sound_stream_producer.subscribe(),
                        self.config.clone().unwrap(),
                        self.stop_connection_producer.subscribe(),
                        self.connection_cleaning_status_producer.clone(),
                    ));
                    tokio::spawn(recording::record(
                        self.sound_stream_producer.clone(),
                        self.stop_recording_producer.subscribe(),
                    ));
                    self.are_we_recovering = false;
                }
            }
            Message::StopStreaming => {
                if !self.are_we_recovering && self.are_we_streaming {
                    println!("Stop Stream");
                    self.are_we_recovering = true;
                    self.are_we_streaming = false;
                    let _ = self.connection_cleaning_status_producer.send(true);
                    let _ = self.stop_connection_producer.send(true);
                    let _ = self.stop_recording_producer.send(true);
                    while !self.connection_cleaning_status_producer.is_empty() {}
                    self.are_we_recovering = false;
                }
            }
            Message::ConfigLoad(config) => {
                self.config = Some(config);
            }
        }
    }
    pub fn view(&self) -> Container<Message> {
        let column = match self.are_we_streaming {
            true => match self.are_we_recovering {
                true => {
                    column![button("Stop Streaming").width(100),]
                }
                false => {
                    column![button("Stop Streaming")
                        .width(100)
                        .on_press(Message::StopStreaming),]
                }
            },
            false => match self.are_we_recovering {
                true => {
                    column![button("Start Streaming").width(100),]
                }
                false => {
                    column![button("Start Streaming")
                        .width(100)
                        .on_press(Message::StartStreaming),]
                }
            },
        };
        container(column)
            .width(200)
            .height(200)
            .center_x()
            .center_y()
    }
    pub fn load_config() -> Command<Message> {
        Command::perform(get_config(), Message::ConfigLoad)
    }
}
