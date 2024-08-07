use std::{fs::File, io::Read};

use crate::Config;

pub fn get_config() -> Config {
    let mut config_file = File::open("configs/streamer_configs.txt").unwrap();
    let mut configs_unparsed = String::new();
    config_file.read_to_string(&mut configs_unparsed).unwrap();

    let configs_parsed: Vec<&str> = configs_unparsed.split_terminator("\n").collect();
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
