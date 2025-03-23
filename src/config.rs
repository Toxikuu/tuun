// config.rs
//
// responsible for handling config.toml

use serde::Deserialize;
use std::{env, fs};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn, error};

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(default)]
    pub lastfm: LastFMConfig,

    #[serde(default)]
    pub discord: DiscordConfig,

    #[serde(default)]
    pub general: GeneralConfig,

    #[serde(default)]
    pub tuunfm: TuunFMConfig,
}

#[derive(Deserialize, Debug, Default)]
pub struct LastFMConfig {
    pub used: bool,
    pub apikey: String,
    pub secret: String,
    pub user: String,
    pub password: String,
}

#[derive(Deserialize, Debug, Default)]
pub struct DiscordConfig {
    #[serde(default = "default_discord_used")]
    pub used: bool,

    #[serde(default = "default_discord_client_id")]
    pub client_id: String,
}

const fn default_discord_used() -> bool { true }

fn default_discord_client_id() -> String {
    "1272345557276295310".to_owned()
}

#[derive(Deserialize, Debug, Default)]
pub struct GeneralConfig {
    #[serde(default = "default_shuffle")]
    pub shuffle: bool,

    #[serde(default = "default_playlist")]
    pub playlist: String,

    #[serde(default = "default_music_dir")]
    pub music_dir: String,
}

const fn default_shuffle() -> bool { true }

fn default_playlist() -> String {
    "/tmp/tuun/all.tpl".to_owned()
}

fn default_music_dir() -> String {
    format!("{}/Music", env::var("HOME").expect("Couldn't find home directory ($HOME is not set)"))
}

#[derive(Deserialize, Debug, Default)]
pub struct TuunFMConfig {
    #[serde(default)]
    pub used: bool,

    #[serde(default = "default_tuunfm_link")]
    pub link: String,
}

fn default_tuunfm_link() -> String {
    "http://127.0.0.1:8080".to_owned()
}

impl Config {
    pub fn load() -> Self {
        let home_dir = PathBuf::from(env::var("HOME").expect("Couldn't find home directory ($HOME is not set)"));
        let home_dir_str = home_dir.to_string_lossy();
        debug!("Detected home directory: {home_dir:?}");

        let config_dir = home_dir.join(".config").join("tuun");
        let config_path = config_dir.join("config.toml");

        if !config_path.exists() {
            info!("Default config does not exist");
            info!("Creating it...");
            Self::create_default(&home_dir, &config_path);
        }

        let Ok(config_str) = fs::read_to_string(&config_path) else {
            error!("Couldn't find config.toml at {config_path:?} despite creating it");
            panic!()
        };

        let mut config: Self = match toml::de::from_str(&config_str) {
            Ok(c) => c,
            Err(e) => {
                error!("Invalid syntax detected in config");
                panic!("{e}");
            }
        };

        // allow ~ in paths in config.toml
        config.general.music_dir = config.general.music_dir.replacen('~', &home_dir_str, 1);
        config.general.playlist  = config.general.playlist .replacen('~', &home_dir_str, 1);

        // allow $HOME in paths in config.toml
        config.general.music_dir = config.general.music_dir.replacen("$HOME", &home_dir_str, 1);
        config.general.playlist  = config.general.playlist .replacen("$HOME", &home_dir_str, 1);

        info!("Loaded config");
        debug!("Config: {config:#?}");
        config
    }

    fn create_default(config_dir: &Path, config_path: &Path) {
        let default_config_path = PathBuf::from("/usr/share/tuun/default_config.toml");
        info!("Copying default config from {default_config_path:?} to {config_path:?}");

        if !config_dir.exists() {
            fs::create_dir_all(config_dir).expect("Failed to create config directory");
            info!("Created config directory at {config_dir:?}");
        }

        if let Err(e) = fs::copy(&default_config_path, config_path) {
            error!("Failed to copy default config from {default_config_path:?} to {config_path:?}: {e}");
            warn!("Did you run make install?")
        }
    }
}
