pub mod gui;
pub mod gui_components;
pub mod gui_utils;
pub mod playing;
pub mod recording;
pub mod streaming;
pub mod utils;

pub const BUFFER_LENGTH: usize = 1000000;
pub const AUDIO_PATH: &str = "audio";
pub const AUDIO_BUFFER_SIZE: usize = 1048576;
pub const AUDIO_SCROLLABLE_BUTTON_SIZE: u16 = 35;

#[derive(Debug, Clone)]
pub struct Config {
    pub address: String,
    pub quality: u8,
    pub latency: u16,
    pub tls: bool,
}
