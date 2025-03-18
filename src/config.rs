// config.rs
//
// responsible for handling config.toml

use serde::Deserialize;
use std::{env, fs};
use std::path::{Path, PathBuf};

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
        let home_dir = PathBuf::from(env::var("HOME").expect("Couldn't find home directory ($HOME is not set)"));
        let home_dir_str = home_dir.to_string_lossy();

        let config_dir = home_dir.join(".config").join("tuun");
        let config_path = config_dir.join("config.toml");

        if !config_path.exists() {
            Self::create_default(&home_dir, &config_path);
        }

        let config_str = fs::read_to_string(config_path).expect("Couldn't find config.toml (even after creating it)");
        let mut config: Self = toml::de::from_str(&config_str).expect("Invalid syntax in config");

        // allow ~ in paths in config.toml
        config.general.music_dir = config.general.music_dir.replacen('~', &home_dir_str, 1);
        config.general.playlist  = config.general.playlist .replacen('~', &home_dir_str, 1);

        // allow $HOME in paths in config.toml
        config.general.music_dir = config.general.music_dir.replacen("$HOME", &home_dir_str, 1);
        config.general.playlist  = config.general.playlist .replacen("$HOME", &home_dir_str, 1);

        config
    }

    fn create_default(config_dir: &Path, config_path: &Path) {
        let default_config_path = PathBuf::from("/usr/share/tuun/default_config.toml");

        if !config_dir.exists() {
            fs::create_dir_all(config_dir).expect("Failed to create config directory")
        }

        fs::copy(default_config_path, config_path).expect("Failed to copy default config (did you run make install?)");
    }
}
