use std::time::Duration;

use streamer::{recording::recording, streaming::start, BUFFER_LENGTH};
use tokio::sync::broadcast::channel;

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    let (sound_stream_producer, sound_stream_consumer) = channel(BUFFER_LENGTH);
    tokio::spawn(recording(sound_stream_producer));
    tokio::spawn(start(sound_stream_consumer));
    tokio::time::sleep(Duration::from_secs(1000000000)).await;
}
