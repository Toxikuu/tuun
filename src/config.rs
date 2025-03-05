// config.rs
//
// responsible for handling config.toml

use serde::Deserialize;
use std::{env, fs, path::PathBuf};

#[derive(Deserialize, Debug)]
pub struct Config {
    pub lastfm: LastFMConfig,
    pub discord: DiscordConfig,
    pub general: GeneralConfig,
    pub tuunfm: TuunFMConfig,
}

#[derive(Deserialize, Debug)]
pub struct LastFMConfig {
    pub used: bool,
    pub apikey: String,
    pub secret: String,
    pub user: String,
    pub password: String,
}

#[derive(Deserialize, Debug)]
pub struct DiscordConfig {
    pub used: bool,
    pub client_id: String,
}

#[derive(Deserialize, Debug)]
pub struct GeneralConfig {
    pub verbose: bool,
    pub socket: String,
    pub music_dir: String,
    pub playlist: String,
}

#[derive(Deserialize, Debug)]
pub struct TuunFMConfig {
    pub used: bool,
    pub link: String,
}

impl Config {
    pub fn load() -> Self {
        let home_dir = PathBuf::from(env::var("HOME").expect("Couldn't find home directory"));
        let config_path = home_dir.join(".config/tuun/config.toml");
        let config_str = fs::read_to_string(config_path).expect("Couldn't find config.toml");
        toml::de::from_str(&config_str).expect("Invalid config")
    }
}
