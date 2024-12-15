// config.rs
//
// responsible for handling config.toml

use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
pub struct Config {
    pub lastfm: LastFMConfig,
    pub discord: DiscordConfig,
    pub general: GeneralConfig,
    pub tuunfm: TuunFMConfig,
}

#[derive(Deserialize)]
pub struct LastFMConfig {
    pub used: bool,
    pub apikey: String,
    pub secret: String,
    pub user: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct DiscordConfig {
    pub used: bool,
    pub client_id: String,
}

#[derive(Deserialize)]
pub struct GeneralConfig {
    pub verbose: bool,
    pub socket: String,
    pub playlist: String,
    pub polling_rate: u64,
}

#[derive(Deserialize)]
pub struct TuunFMConfig {
    pub used: bool,
    pub link: String,
}

impl Config {
    pub fn load() -> Self {
        let home_dir = dirs::home_dir().expect("Couldn't find home directory");
        let config_path = home_dir.join(".config/tuun/config.toml");
        let config_str = fs::read_to_string(config_path).expect("Couldn't find config.toml");
        let config: Config = toml::de::from_str(&config_str).expect("Invalid config");

        config
    }
}
