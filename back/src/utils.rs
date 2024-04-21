use tokio::{fs::File, io::AsyncReadExt};

use crate::Config;

pub async fn get_config() -> Config {
    let mut config_file = File::open("configs/relay_configs.txt").await.unwrap();
    let mut configs_unparsed = String::new();
    config_file
        .read_to_string(&mut configs_unparsed)
        .await
        .unwrap();

    let configs_parsed: Vec<&str> = configs_unparsed.split_terminator("\n").collect();
    let mut configs_cleaned: Vec<&str> = vec![];

    for config in configs_parsed {
        let dirty_configs: Vec<&str> = config.split(": ").collect();
        configs_cleaned.push(dirty_configs[1]);
    }
    Config {
        axum_address: configs_cleaned[0].to_string(),
        listener_address: configs_cleaned[1].to_string(),
        streamer_address: configs_cleaned[2].to_string(),
        latency: configs_cleaned[3].parse().unwrap(),
        tls: configs_cleaned[4].parse().unwrap(),
    }
}
