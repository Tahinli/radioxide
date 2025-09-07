pub mod recording;
pub mod streaming;

pub const BUFFER_LENGTH: usize = 1000000;

pub struct Config {
    pub address: String,
    pub quality: u8,
    pub latency: u16,
    pub tls: bool,
}
