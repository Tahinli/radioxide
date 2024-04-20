use std::time::Duration;

use streamer::{recording::recording, streaming::start, Config, BUFFER_LENGTH};
use tokio::{fs::File, io::AsyncReadExt, sync::broadcast::channel};

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    let streamer_config = get_config().await;
    let (sound_stream_producer, sound_stream_consumer) = channel(BUFFER_LENGTH);
    tokio::spawn(recording(sound_stream_producer));
    tokio::spawn(start(sound_stream_consumer, streamer_config));
    loop {
        tokio::time::sleep(Duration::from_secs(1000000000)).await;
    }
}

async fn get_config() -> Config {
    let mut config_file = File::open("configs/streamer_configs.txt").await.unwrap();
    let mut configs_unparsed = String::new();
    config_file.read_to_string(&mut configs_unparsed).await.unwrap();

    let configs_parsed:Vec<&str> = configs_unparsed.split_terminator("\n").collect();
    let mut configs_cleaned: Vec<&str> = vec![];

    for config in configs_parsed {
        let dirty_configs: Vec<&str> = config.split(": ").collect();
        configs_cleaned.push(dirty_configs[1]);
    }
    Config {
        address: configs_cleaned[0].to_string(),
        quality: configs_cleaned[1].parse().unwrap(),
        latency: configs_cleaned[2].parse().unwrap(),
        tls: configs_cleaned[3].parse().unwrap(),
    }
}
