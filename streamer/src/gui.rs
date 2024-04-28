use iced::{
    widget::{container, row, Container},
    Command,
};
use tokio::sync::broadcast::{channel, Sender};

use crate::{
    gui_utils::button_with_centered_text, recording, streaming, utils::get_config, Config, BUFFER_LENGTH
};
#[derive(Debug, Clone)]
pub enum Event {
    Connect,
    Disconnect,
    Record,
    StopRecord,
    PlayAudio,
    StopAudio,
    LoadConfig(Config),
}
#[derive(Debug, Clone)]

pub enum State {
    Connected,
    Disconnected,
    Recording,
    StopRecording,
    PlayingAudio,
    StopAudio,
}

#[derive(Debug, Clone)]
pub enum Message {
    Event(Event),
    State(State),
}
#[derive(Debug)]
struct DataChannel {
    sound_stream_sender: Sender<f32>,
}
#[derive(Debug)]
struct CommunicationChannel {
    base_to_streaming: Sender<bool>,
    streaming_to_base: Sender<bool>,
    base_to_recording: Sender<bool>,
    recording_to_base: Sender<bool>,
    base_to_playing: Sender<bool>,
    playing_to_base: Sender<bool>,
}
#[derive(Debug)]
enum Condition {
    Active,
    Loading,
    Passive,
}

#[derive(Debug)]
struct GUIStatus {
    are_we_connect: Condition,
    are_we_record: Condition,
    are_we_play_audio: Condition,
}
#[derive(Debug)]
pub struct Streamer {
    config: Option<Config>,
    data_channel: DataChannel,
    communication_channel: CommunicationChannel,
    gui_status: GUIStatus,
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
            data_channel: DataChannel {
                sound_stream_sender: channel(BUFFER_LENGTH).0,
            },
            communication_channel: CommunicationChannel {
                base_to_streaming: channel(1).0,
                streaming_to_base: channel(1).0,
                base_to_recording: channel(1).0,
                recording_to_base: channel(1).0,
                base_to_playing: channel(1).0,
                playing_to_base: channel(1).0,
            },
            gui_status: GUIStatus {
                are_we_connect: Condition::Passive,
                are_we_record: Condition::Passive,
                are_we_play_audio: Condition::Passive,
            },
        }
    }
    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Event(event) => match event {
                Event::Connect => {
                    println!("Connect");
                    self.gui_status.are_we_connect = Condition::Loading;
                    let mut streaming_to_base_receiver =
                        self.communication_channel.streaming_to_base.subscribe();
                    tokio::spawn(streaming::connect(
                        self.data_channel.sound_stream_sender.subscribe(),
                        self.config.clone().unwrap(),
                        self.communication_channel.base_to_streaming.subscribe(),
                        self.communication_channel.streaming_to_base.clone(),
                    ));
                    Command::perform(
                        async move {
                            match streaming_to_base_receiver.recv().await {
                                Ok(_) => State::Connected,
                                Err(err_val) => {
                                    eprintln!("Error: Communication | {}", err_val);
                                    State::Disconnected
                                }
                            }
                        },
                        Message::State,
                    )
                }
                Event::Disconnect => {
                    println!("Disconnect");
                    self.gui_status.are_we_connect = Condition::Loading;
                    let mut streaming_to_base_receiver =
                        self.communication_channel.streaming_to_base.subscribe();
                    let _ = self.communication_channel.base_to_streaming.send(false);
                    Command::perform(
                        async move {
                            match streaming_to_base_receiver.recv().await {
                                Ok(_) => State::Disconnected,
                                Err(err_val) => {
                                    eprintln!("Error: Communication | {}", err_val);
                                    State::Connected
                                }
                            }
                        },
                        Message::State,
                    )
                }
                Event::Record => {
                    println!("Record");
                    self.gui_status.are_we_record = Condition::Loading;
                    let mut recording_to_base_receiver =
                        self.communication_channel.recording_to_base.subscribe();
                    tokio::spawn(recording::record(
                        self.data_channel.sound_stream_sender.clone(),
                        self.communication_channel.base_to_recording.subscribe(),
                        self.communication_channel.recording_to_base.clone(),
                    ));
                    Command::perform(
                        async move {
                            match recording_to_base_receiver.recv().await {
                                Ok(_) => State::Recording,
                                Err(err_val) => {
                                    eprintln!("Error: Communication | Streaming | {}", err_val);
                                    State::StopRecording
                                }
                            }
                        },
                        Message::State,
                    )
                }
                Event::StopRecord => {
                    println!("Stop Record");
                    self.gui_status.are_we_record = Condition::Loading;
                    let mut recording_to_base_receiver =
                        self.communication_channel.recording_to_base.subscribe();
                    let _ = self.communication_channel.base_to_recording.send(false);
                    Command::perform(
                        async move {
                            match recording_to_base_receiver.recv().await {
                                Ok(_) => State::StopRecording,
                                Err(err_val) => {
                                    eprintln!("Error: Communication | Recording | {}", err_val);
                                    State::Recording
                                }
                            }
                        },
                        Message::State,
                    )
                }
                Event::PlayAudio => {
                    println!("Play Audio");
                    self.gui_status.are_we_play_audio = Condition::Loading;
                    let mut playing_to_base_receiver =
                        self.communication_channel.playing_to_base.subscribe();
                    //tokio::spawn(future);
                    Command::perform(
                        async move {
                            match playing_to_base_receiver.recv().await {
                                Ok(_) => State::PlayingAudio,
                                Err(err_val) => {
                                    eprint!("Error: Communication | Playing | {}", err_val);
                                    State::StopAudio
                                }
                            }
                        },
                        Message::State,
                    )
                }
                Event::StopAudio => {
                    println!("Stop Audio");
                    self.gui_status.are_we_play_audio = Condition::Loading;
                    let mut playing_to_base_receiver =
                        self.communication_channel.playing_to_base.subscribe();
                    let _ = self.communication_channel.base_to_playing.send(false);
                    Command::perform(
                        async move {
                            match playing_to_base_receiver.recv().await {
                                Ok(_) => State::StopAudio,
                                Err(err_val) => {
                                    eprint!("Error: Communication | Playing | {}", err_val);
                                    State::PlayingAudio
                                }
                            }
                        },
                        Message::State,
                    )
                }
                Event::LoadConfig(config) => {
                    self.config = Some(config);
                    Command::none()
                }
            },
            Message::State(state) => match state {
                State::Connected => {
                    self.gui_status.are_we_connect = Condition::Active;
                    Command::none()
                }
                State::Disconnected => {
                    self.gui_status.are_we_connect = Condition::Passive;
                    Command::none()
                }
                State::Recording => {
                    self.gui_status.are_we_record = Condition::Active;
                    Command::none()
                }
                State::StopRecording => {
                    self.gui_status.are_we_record = Condition::Passive;
                    Command::none()
                }
                State::PlayingAudio => {
                    self.gui_status.are_we_play_audio = Condition::Active;
                    Command::none()
                }
                State::StopAudio => {
                    self.gui_status.are_we_play_audio = Condition::Passive;
                    Command::none()
                }
            },
        }
    }
    pub fn view(&self) -> Container<Message> {
        let connect_button = match self.gui_status.are_we_connect {
            Condition::Active => {
                button_with_centered_text("Disconnect").on_press(Message::Event(Event::Disconnect))
            }
            Condition::Loading => button_with_centered_text("Processing"),
            Condition::Passive => {
                button_with_centered_text("Connect").on_press(Message::Event(Event::Connect))
            }
        };

        let record_button = match self.gui_status.are_we_record {
            Condition::Active => {
                button_with_centered_text("Stop Record").on_press(Message::Event(Event::StopRecord))
            }
            Condition::Loading => button_with_centered_text("Processing"),
            Condition::Passive => {
                button_with_centered_text("Record").on_press(Message::Event(Event::Record))
            }
        };

        let play_audio_button =
            match self.gui_status.are_we_play_audio {
                Condition::Active => button_with_centered_text("Stop Audio")
                    .on_press(Message::Event(Event::StopAudio)),
                Condition::Loading => button_with_centered_text("Processing"),
                Condition::Passive => button_with_centered_text("Play Audio")
                    .on_press(Message::Event(Event::PlayAudio)),
            };

        let content = row![connect_button, record_button, play_audio_button]
            .spacing(20)
            .width(400)
            .height(35);
        container(content).height(300).center_x().center_y()
    }
    pub fn load_config() -> Command<Message> {
        Command::perform(
            async move {
                let config = get_config().await;
                Event::LoadConfig(config)
            },
            Message::Event,
        )
    }
}
