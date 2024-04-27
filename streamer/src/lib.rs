pub mod recording;
pub mod streaming;
pub mod utils;
pub mod gui;

pub const BUFFER_LENGTH: usize = 1000000;

#[derive(Debug, Clone)]
pub struct Config {
    pub address: String,
    pub quality: u8,
    pub latency: u16,
    pub tls: bool,
}
