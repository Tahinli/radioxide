use std::{cmp::max, fs::File, path::Path, sync::Arc};

use iced::{
    alignment,
    widget::{column, container, row, scrollable, slider, text::LineHeight, Container, Rule},
    window, Color, Command, Length, Subscription,
};
use tokio::sync::{
    broadcast::{channel, Receiver, Sender},
    Mutex,
};

use crate::{
    gui_components::{button_with_centered_text, text_centered},
    gui_utils::{self, change_audio_volume, change_microphone_volume},
    utils::get_config,
    Config, BUFFER_LENGTH,
};

const AUDIOS_PATH: &str = "audios";

#[derive(Debug, Clone)]
pub enum Player {
    Play,
    Pause,
    Stop,
}

#[derive(Debug, Clone)]
struct Features {
    stream: bool,
    record: bool,
    play_audio: bool,
}

#[derive(Debug)]
struct AudioMiscellaneous {
    file: Option<File>,
    selected_file_name: String,
    playing_file_name: String,
    files: Option<Vec<String>>,
    decoded_to_playing_sender: Option<Sender<f32>>,
}

#[derive(Debug, Clone)]
pub enum Event {
    None,
    Connect,
    Disconnect,
    Record,
    StopRecord,
    PlayAudio,
    StopAudio,
    PauseAudio,
    ContinueAudio,
    ChooseAudio(String),
    ChangeMicrophoneVolume(f32),
    ChangeAudioVolume(f32),
    LoadConfig(Config),
    ListFiles(Option<Vec<String>>),
    IcedEvent(iced::Event),
    CloseWindow(window::Id),
}
#[derive(Debug, Clone)]

pub enum State {
    None,
    Connected,
    Disconnected,
    Recording,
    StopRecording,
    PlayingAudio,
    StopAudio,
    PausedAudio,
    ContinuedAudio,
    MicrophoneVolumeChanged,
    AudioVolumeChanged,
}

#[derive(Debug, Clone)]
pub enum Message {
    Event(Event),
    State(State),
}
#[derive(Debug)]
struct DataChannel {
    microphone_stream_sender: Sender<f32>,
    audio_stream_sender: Sender<f32>,
}
#[derive(Debug)]
struct CommunicationChannel {
    base_to_streaming_sender: Sender<bool>,
    streaming_to_base_sender: Sender<bool>,
    base_to_recording_sender: Sender<bool>,
    recording_to_base_sender: Sender<bool>,
    base_to_playing_sender: Sender<Player>,
    playing_to_base_sender: Sender<Player>,
}
#[derive(Debug, PartialEq)]
enum Condition {
    Active,
    Loading,
    Passive,
}

#[derive(Debug)]
struct ChangeableValue {
    value: Arc<Mutex<f32>>,
}

#[derive(Debug)]
struct GUIStatus {
    are_we_connect: Condition,
    are_we_record: Condition,
    are_we_play_audio: Condition,
    are_we_paused_audio: Condition,
    microphone_volume: ChangeableValue,
    audio_volume: ChangeableValue,
}
#[derive(Debug)]
pub struct Streamer {
    config: Option<Config>,
    data_channel: DataChannel,
    communication_channel: CommunicationChannel,
    audio_miscellaneous: AudioMiscellaneous,
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
                microphone_stream_sender: channel(BUFFER_LENGTH).0,
                audio_stream_sender: channel(BUFFER_LENGTH).0,
            },
            communication_channel: CommunicationChannel {
                base_to_streaming_sender: channel(1).0,
                streaming_to_base_sender: channel(1).0,
                base_to_recording_sender: channel(1).0,
                recording_to_base_sender: channel(1).0,
                base_to_playing_sender: channel(1).0,
                playing_to_base_sender: channel(1).0,
            },
            audio_miscellaneous: AudioMiscellaneous {
                file: None,
                selected_file_name: String::new(),
                playing_file_name: String::new(),
                files: None,
                decoded_to_playing_sender: None,
            },
            gui_status: GUIStatus {
                are_we_connect: Condition::Passive,
                are_we_record: Condition::Passive,
                are_we_play_audio: Condition::Passive,
                are_we_paused_audio: Condition::Passive,
                microphone_volume: ChangeableValue {
                    value: Arc::new(1.0.into()),
                },
                audio_volume: ChangeableValue {
                    value: Arc::new(1.0.into()),
                },
            },
        }
    }
    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Event(event) => match event {
                Event::None => Command::none(),
                Event::Connect => {
                    println!("Connect");
                    self.gui_status.are_we_connect = Condition::Loading;
                    let microphone_stream_receiver =
                        self.data_channel.microphone_stream_sender.subscribe();
                    let audio_stream_receiver = self.data_channel.audio_stream_sender.subscribe();
                    let streamer_config = self.config.clone().unwrap();
                    let streaming_to_base_sender =
                        self.communication_channel.streaming_to_base_sender.clone();
                    let base_to_streaming_receiver = self
                        .communication_channel
                        .base_to_streaming_sender
                        .subscribe();
                    let microphone_stream_volume = self.gui_status.microphone_volume.value.clone();
                    let audio_stream_volume = self.gui_status.audio_volume.value.clone();
                    Command::perform(
                        async move {
                            gui_utils::connect(
                                microphone_stream_receiver,
                                audio_stream_receiver,
                                streamer_config,
                                streaming_to_base_sender,
                                base_to_streaming_receiver,
                                microphone_stream_volume,
                                audio_stream_volume,
                            )
                            .await
                        },
                        Message::State,
                    )
                }
                Event::Disconnect => {
                    println!("Disconnect");
                    self.gui_status.are_we_connect = Condition::Loading;

                    let streaming_to_base_receiver = self
                        .communication_channel
                        .streaming_to_base_sender
                        .subscribe();
                    let base_to_streaming_sender =
                        self.communication_channel.base_to_streaming_sender.clone();

                    Command::perform(
                        async move {
                            gui_utils::disconnect(
                                streaming_to_base_receiver,
                                base_to_streaming_sender,
                            )
                            .await
                        },
                        Message::State,
                    )
                }
                Event::Record => {
                    println!("Record");
                    self.gui_status.are_we_record = Condition::Loading;

                    let microphone_stream_sender =
                        self.data_channel.microphone_stream_sender.clone();
                    let recording_to_base_sender =
                        self.communication_channel.recording_to_base_sender.clone();
                    let base_to_recording_receiver = self
                        .communication_channel
                        .base_to_recording_sender
                        .subscribe();

                    Command::perform(
                        async move {
                            gui_utils::start_recording(
                                microphone_stream_sender,
                                recording_to_base_sender,
                                base_to_recording_receiver,
                            )
                            .await
                        },
                        Message::State,
                    )
                }
                Event::StopRecord => {
                    println!("Stop Record");
                    self.gui_status.are_we_record = Condition::Loading;
                    let recording_to_base_receiver = self
                        .communication_channel
                        .recording_to_base_sender
                        .subscribe();
                    let base_to_recording_sender =
                        self.communication_channel.base_to_recording_sender.clone();
                    Command::perform(
                        async move {
                            gui_utils::stop_recording(
                                recording_to_base_receiver,
                                base_to_recording_sender,
                            )
                            .await
                        },
                        Message::State,
                    )
                }
                Event::PlayAudio => {
                    println!("Play Audio");
                    self.gui_status.are_we_play_audio = Condition::Loading;
                    let path = format!(
                        "{}/{}",
                        AUDIOS_PATH, self.audio_miscellaneous.selected_file_name
                    );
                    match File::open(path) {
                        Ok(file) => {
                            self.audio_miscellaneous.file = Some(file);
                        }
                        Err(err_val) => {
                            eprintln!("Error: Open File | {}", err_val);
                            self.audio_miscellaneous.file = None;
                            self.gui_status.are_we_play_audio = Condition::Passive;
                            return Command::none();
                        }
                    }
                    self.audio_miscellaneous.decoded_to_playing_sender = Some(
                        channel(max(
                            self.audio_miscellaneous
                                .file
                                .as_ref()
                                .unwrap()
                                .metadata()
                                .unwrap()
                                .len() as usize
                                * 4,
                            1,
                        ))
                        .0,
                    );

                    let audio_stream_sender = self.data_channel.audio_stream_sender.clone();
                    let playing_to_base_sender =
                        self.communication_channel.playing_to_base_sender.clone();
                    let base_to_playing_receiver = self
                        .communication_channel
                        .base_to_playing_sender
                        .subscribe();

                    let playing_to_base_receiver_is_audio_finished = self
                        .communication_channel
                        .playing_to_base_sender
                        .subscribe();

                    let playing_to_base_receiver_is_audio_stopped = self
                        .communication_channel
                        .playing_to_base_sender
                        .subscribe();

                    let base_to_playing_sender =
                        self.communication_channel.base_to_playing_sender.clone();

                    let file = self
                        .audio_miscellaneous
                        .file
                        .as_ref()
                        .unwrap()
                        .try_clone()
                        .unwrap();
                    let decoded_to_playing_sender_for_playing = self
                        .audio_miscellaneous
                        .decoded_to_playing_sender
                        .clone()
                        .unwrap();

                    let decoded_to_playing_sender_for_is_finished = self
                        .audio_miscellaneous
                        .decoded_to_playing_sender
                        .clone()
                        .unwrap();

                    let audio_volume = self.gui_status.audio_volume.value.clone();

                    let playing_command = Command::perform(
                        async move {
                            gui_utils::start_playing(
                                audio_stream_sender,
                                decoded_to_playing_sender_for_playing,
                                file,
                                playing_to_base_sender,
                                base_to_playing_receiver,
                                audio_volume,
                            )
                            .await
                        },
                        Message::State,
                    );
                    let is_finished_command = Command::perform(
                        async move {
                            gui_utils::is_playing_finished(
                                playing_to_base_receiver_is_audio_finished,
                                playing_to_base_receiver_is_audio_stopped,
                                base_to_playing_sender,
                                decoded_to_playing_sender_for_is_finished,
                            )
                            .await
                        },
                        Message::State,
                    );
                    let commands = vec![playing_command, is_finished_command];
                    Command::batch(commands)
                }
                Event::StopAudio => {
                    println!("Stop Audio");
                    self.gui_status.are_we_play_audio = Condition::Loading;

                    let playing_to_base_receiver = self
                        .communication_channel
                        .playing_to_base_sender
                        .subscribe();
                    let base_to_playing_sender =
                        self.communication_channel.base_to_playing_sender.clone();

                    Command::perform(
                        async move {
                            gui_utils::stop_playing(
                                playing_to_base_receiver,
                                base_to_playing_sender,
                            )
                            .await
                        },
                        Message::State,
                    )
                }
                Event::PauseAudio => {
                    println!("Pause Audio");
                    self.gui_status.are_we_paused_audio = Condition::Loading;

                    let playing_to_base_receiver = self
                        .communication_channel
                        .playing_to_base_sender
                        .subscribe();
                    let base_to_playing_sender =
                        self.communication_channel.base_to_playing_sender.clone();

                    Command::perform(
                        async move {
                            gui_utils::pause_playing(
                                playing_to_base_receiver,
                                base_to_playing_sender,
                            )
                            .await
                        },
                        Message::State,
                    )
                }
                Event::ContinueAudio => {
                    println!("Continue Audio");
                    self.gui_status.are_we_paused_audio = Condition::Loading;

                    let playing_to_base_receiver = self
                        .communication_channel
                        .playing_to_base_sender
                        .subscribe();
                    let base_to_playing_sender =
                        self.communication_channel.base_to_playing_sender.clone();

                    Command::perform(
                        async move {
                            gui_utils::continue_playing(
                                playing_to_base_receiver,
                                base_to_playing_sender,
                            )
                            .await
                        },
                        Message::State,
                    )
                }
                Event::ChooseAudio(chosen_audio) => {
                    let path = format!("{}/{}", AUDIOS_PATH, chosen_audio);
                    match File::open(path) {
                        Ok(file) => {
                            self.audio_miscellaneous.file = Some(file);
                            self.audio_miscellaneous.selected_file_name = chosen_audio;
                        }
                        Err(err_val) => {
                            eprintln!("Error: Select Open | {}", err_val);
                            self.audio_miscellaneous.file = None;
                        }
                    }
                    Command::none()
                }
                Event::ChangeMicrophoneVolume(value) => {
                    *self.gui_status.microphone_volume.value.blocking_lock() = value;
                    let microphone_volume = self.gui_status.microphone_volume.value.clone();
                    Command::perform(
                        async move { change_microphone_volume(value, microphone_volume).await },
                        Message::State,
                    )
                }
                Event::ChangeAudioVolume(value) => {
                    *self.gui_status.audio_volume.value.blocking_lock() = value;
                    let audio_volume = self.gui_status.audio_volume.value.clone();
                    Command::perform(
                        async move { change_audio_volume(value, audio_volume).await },
                        Message::State,
                    )
                }
                Event::LoadConfig(config) => {
                    self.config = Some(config);
                    Command::none()
                }
                Event::ListFiles(files) => {
                    self.audio_miscellaneous.files = files;
                    Command::none()
                }
                Event::IcedEvent(iced_event) => match iced_event {
                    iced::Event::Keyboard(_) => Command::perform(
                        async move {
                            let files = gui_utils::list_files(Path::new(AUDIOS_PATH)).await;
                            Event::ListFiles(files)
                        },
                        Message::Event,
                    ),
                    iced::Event::Mouse(_) => Command::perform(
                        async move {
                            let files = gui_utils::list_files(Path::new(AUDIOS_PATH)).await;
                            Event::ListFiles(files)
                        },
                        Message::Event,
                    ),
                    iced::Event::Window(id, window_event) => {
                        if let window::Event::CloseRequested = window_event {
                            self.exit(id)
                        } else {
                            Command::perform(
                                async move {
                                    let files = gui_utils::list_files(Path::new(AUDIOS_PATH)).await;
                                    Event::ListFiles(files)
                                },
                                Message::Event,
                            )
                        }
                    }
                    iced::Event::Touch(_) => Command::perform(
                        async move {
                            let files = gui_utils::list_files(Path::new(AUDIOS_PATH)).await;
                            Event::ListFiles(files)
                        },
                        Message::Event,
                    ),
                    iced::Event::PlatformSpecific(_) => Command::perform(
                        async move {
                            let files = gui_utils::list_files(Path::new(AUDIOS_PATH)).await;
                            Event::ListFiles(files)
                        },
                        Message::Event,
                    ),
                },
                Event::CloseWindow(id) => window::close(id),
            },
            Message::State(state) => match state {
                State::None => Command::none(),
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
                    self.audio_miscellaneous.playing_file_name =
                        self.audio_miscellaneous.selected_file_name.clone();
                    self.gui_status.are_we_play_audio = Condition::Active;
                    self.gui_status.are_we_paused_audio = Condition::Passive;
                    Command::none()
                }
                State::StopAudio => {
                    self.audio_miscellaneous.playing_file_name = String::new();
                    self.gui_status.are_we_play_audio = Condition::Passive;
                    Command::none()
                }
                State::PausedAudio => {
                    self.gui_status.are_we_paused_audio = Condition::Active;
                    Command::none()
                }
                State::ContinuedAudio => {
                    self.gui_status.are_we_paused_audio = Condition::Passive;
                    Command::none()
                }
                State::MicrophoneVolumeChanged => Command::none(),
                State::AudioVolumeChanged => Command::none(),
            },
        }
    }
    pub fn view(&self) -> Container<Message> {
        //let color_red = Color::from_rgb8(255, 0, 0);
        let color_green = Color::from_rgb8(0, 255, 0);
        let color_blue = Color::from_rgb8(0, 0, 255);
        let color_yellow = Color::from_rgb8(255, 255, 0);
        //let color_white = Color::from_rgb8(255, 255, 255);
        let color_grey = Color::from_rgb8(128, 128, 128);
        //let color_black = Color::from_rgb8(0, 0, 0);
        let color_pink = Color::from_rgb8(255, 150, 150);

        let header = text_centered("Radioxide")
            .size(35)
            .line_height(LineHeight::Relative(1.0));

        let connection_text = text_centered("Connection");
        let recording_text = text_centered("Microphone");
        let play_audio_text = text_centered("Play Audio");
        let pause_audio_text = text_centered("Pause Audio");

        let connection_status_text;
        let recording_status_text;
        let play_audio_status_text;
        let paused_audio_status_text;

        let connect_button = match self.gui_status.are_we_connect {
            Condition::Active => {
                connection_status_text = text_centered("Active").color(color_green);
                button_with_centered_text("Disconnect").on_press(Message::Event(Event::Disconnect))
            }
            Condition::Loading => {
                connection_status_text = text_centered("Loading").color(color_yellow);
                button_with_centered_text("Processing")
            }
            Condition::Passive => {
                connection_status_text = text_centered("Passive").color(color_pink);
                button_with_centered_text("Connect").on_press(Message::Event(Event::Connect))
            }
        };

        let record_button = match self.gui_status.are_we_record {
            Condition::Active => {
                recording_status_text = text_centered("Active").color(color_green);
                button_with_centered_text("Stop Mic").on_press(Message::Event(Event::StopRecord))
            }
            Condition::Loading => {
                recording_status_text = text_centered("Loading").color(color_yellow);
                button_with_centered_text("Processing")
            }
            Condition::Passive => {
                recording_status_text = text_centered("Passive").color(color_pink);
                button_with_centered_text("Start Mic").on_press(Message::Event(Event::Record))
            }
        };

        let play_audio_button = match self.gui_status.are_we_play_audio {
            Condition::Active => {
                play_audio_status_text = text_centered("Active").color(color_green);
                button_with_centered_text("Stop Audio").on_press(Message::Event(Event::StopAudio))
            }
            Condition::Loading => {
                play_audio_status_text = text_centered("Loading").color(color_yellow);
                button_with_centered_text("Processing")
            }
            Condition::Passive => match self.audio_miscellaneous.file {
                Some(_) => {
                    play_audio_status_text = text_centered("Passive").color(color_pink);
                    button_with_centered_text("Play Audio")
                        .on_press(Message::Event(Event::PlayAudio))
                }
                None => {
                    play_audio_status_text = text_centered("No Audio").color(color_pink);
                    button_with_centered_text("No Audio")
                }
            },
        };

        let pause_audio_button = if let Condition::Active = self.gui_status.are_we_play_audio {
            match self.gui_status.are_we_paused_audio {
                Condition::Active => {
                    paused_audio_status_text = text_centered("Paused").color(color_blue);
                    button_with_centered_text("Continue Audio")
                        .on_press(Message::Event(Event::ContinueAudio))
                }
                Condition::Loading => {
                    paused_audio_status_text = text_centered("Loading").color(color_yellow);
                    button_with_centered_text("Processing")
                }
                Condition::Passive => {
                    paused_audio_status_text = text_centered("Playing").color(color_yellow);
                    button_with_centered_text("Pause Audio")
                        .on_press(Message::Event(Event::PauseAudio))
                }
            }
        } else {
            paused_audio_status_text = text_centered("Waiting").color(color_grey);
            button_with_centered_text("No Purpose")
        };

        let microphone_volume_slider = slider(
            0.0..=1.0,
            *self.gui_status.microphone_volume.value.blocking_lock(),
            |value| Message::Event(Event::ChangeMicrophoneVolume(value)),
        )
        .step(0.01);

        let audio_volume_slider = slider(
            0.0..=1.0,
            *self.gui_status.audio_volume.value.blocking_lock(),
            |value| Message::Event(Event::ChangeAudioVolume(value)),
        )
        .step(0.01);

        let mut audio_scrollable_content = column![]
            .spacing(1)
            .height(Length::Fill)
            .width(Length::Fill);
        let audio_selected = text_centered(format!(
            "Selected: {}",
            self.audio_miscellaneous.selected_file_name.clone()
        ));
        let audio_playing = text_centered(format!(
            "Playing: {}",
            self.audio_miscellaneous.playing_file_name.clone()
        ));
        if self.audio_miscellaneous.files.is_some() {
            for file in self.audio_miscellaneous.files.as_ref().clone().unwrap() {
                let button = button_with_centered_text(file)
                    .on_press(Message::Event(Event::ChooseAudio(file.to_string())));
                audio_scrollable_content = audio_scrollable_content.push(button.height(35));
            }
        }
        let audios_scrollable = scrollable(audio_scrollable_content)
            .height(Length::Fill)
            .width(Length::Fill);
        let audio_info_content = column![audio_selected, audio_playing,].height(60);
        let header_content = row![header].width(350).height(50);
        let text_content = row![
            connection_text,
            Rule::vertical(1),
            recording_text,
            Rule::vertical(1),
            play_audio_text,
            Rule::vertical(1),
            pause_audio_text,
        ]
        .spacing(5)
        .width(350)
        .height(35);

        let status_content = row![
            connection_status_text,
            Rule::vertical(1),
            recording_status_text,
            Rule::vertical(1),
            play_audio_status_text,
            Rule::vertical(1),
            paused_audio_status_text,
        ]
        .spacing(5)
        .width(350)
        .height(35);
        let button_content = row![
            connect_button,
            record_button,
            play_audio_button,
            pause_audio_button,
        ]
        .spacing(5)
        .width(350)
        .height(35);
        let volume_content = row![microphone_volume_slider, audio_volume_slider,]
            .spacing(5)
            .width(350)
            .height(35);
        let content = column![
            header_content,
            Rule::horizontal(1),
            text_content,
            button_content,
            status_content,
            Rule::horizontal(1),
            volume_content,
            audios_scrollable,
            audio_info_content,
        ]
        .spacing(20)
        .width(Length::Fill)
        .height(Length::Fill);
        container(content)
            .height(Length::Fill)
            .center_x()
            .align_y(alignment::Vertical::Top)
    }
    pub fn subscription(&self) -> Subscription<Message> {
        iced::event::listen()
            .map(Event::IcedEvent)
            .map(Message::Event)
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
    pub fn list_files() -> Command<Message> {
        Command::perform(
            async move {
                let files = gui_utils::list_files(Path::new(AUDIOS_PATH)).await;
                Event::ListFiles(files)
            },
            Message::Event,
        )
    }
    fn call_closer(
        streaming_to_base_receiver: Receiver<bool>,
        base_to_streaming_sender: Sender<bool>,
        recording_to_base_receiver: Receiver<bool>,
        base_to_recording_sender: Sender<bool>,
        playing_to_base_receiver: Receiver<Player>,
        base_to_playing_sender: Sender<Player>,
        features_in_need: Features,
        window_id: window::Id,
    ) -> Command<Message> {
        Command::perform(
            async move {
                if features_in_need.stream {
                    gui_utils::disconnect(streaming_to_base_receiver, base_to_streaming_sender)
                        .await;
                }
                if features_in_need.record {
                    gui_utils::stop_recording(recording_to_base_receiver, base_to_recording_sender)
                        .await;
                }
                if features_in_need.play_audio {
                    gui_utils::stop_playing(playing_to_base_receiver, base_to_playing_sender).await;
                }
                Event::CloseWindow(window_id)
            },
            Message::Event,
        )
    }
    fn exit(&self, window_id: window::Id) -> Command<Message> {
        let mut features_in_need = Features {
            stream: false,
            record: false,
            play_audio: false,
        };

        if self.gui_status.are_we_connect == Condition::Active {
            features_in_need.stream = true;
        }
        if self.gui_status.are_we_record == Condition::Active {
            features_in_need.record = true;
        }
        if self.gui_status.are_we_play_audio == Condition::Active {
            features_in_need.play_audio = true;
        }
        let streaming_to_base_receiver = self
            .communication_channel
            .streaming_to_base_sender
            .subscribe();
        let base_to_streaming_sender = self.communication_channel.base_to_streaming_sender.clone();

        let recording_to_base_receiver = self
            .communication_channel
            .recording_to_base_sender
            .subscribe();
        let base_to_recording_sender = self.communication_channel.base_to_recording_sender.clone();

        let playing_to_base_receiver = self
            .communication_channel
            .playing_to_base_sender
            .subscribe();
        let base_to_playing_sender = self.communication_channel.base_to_playing_sender.clone();

        Self::call_closer(
            streaming_to_base_receiver,
            base_to_streaming_sender,
            recording_to_base_receiver,
            base_to_recording_sender,
            playing_to_base_receiver,
            base_to_playing_sender,
            features_in_need,
            window_id,
        )
    }
}
