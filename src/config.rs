// config.rs
//
// responsible for handling config.toml
//
// NOTE: My dumb ass spent entirely too long failing to figure out how to just use impl Default for
// structs with missing fields. The solution is to just use #[serde(default)] at the struct level.
// https://users.rust-lang.org/t/serde-default-versus-impl-default/66773
// https://serde.rs/container-attrs.html#default

use std::{
    env,
    fs,
    path::{
        Path,
        PathBuf,
    },
};

use serde::Deserialize;
use tracing::{
    debug,
    error,
    info,
    warn,
};

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
pub struct Config {
    pub lastfm:  LastFMConfig,
    pub discord: DiscordConfig,
    pub general: GeneralConfig,
    pub tuunfm:  TuunFMConfig,
}

impl Default for LastFMConfig {
    fn default() -> Self {
        Self {
            used:     false,
            apikey:   String::with_capacity(0),
            secret:   String::with_capacity(0),
            user:     String::with_capacity(0),
            password: String::with_capacity(0),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct LastFMConfig {
    pub used:     bool,
    pub apikey:   String,
    pub secret:   String,
    pub user:     String,
    pub password: String,
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            used:      true,
            client_id: "1272345557276295310".to_owned(),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct DiscordConfig {
    pub used:      bool,
    pub client_id: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            shuffle:       true,
            playlist:      "/tmp/tuun/all.tpl".to_owned(),
            music_dir:     format!("{}/Music", std::env::var("HOME").expect("$HOME not set")),
            recent_length: 200,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct GeneralConfig {
    pub shuffle:       bool,
    pub playlist:      String,
    pub music_dir:     String,
    pub recent_length: usize,
}

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct TuunFMConfig {
    pub used: bool,
    pub link: String,
}

impl Default for TuunFMConfig {
    fn default() -> Self {
        Self {
            used: false,
            link: "http://127.0.0.1:8080".to_owned(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let home_dir = PathBuf::from(
            env::var("HOME").expect("Couldn't find home directory ($HOME is not set)"),
        );
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
            | Ok(c) => c,
            | Err(e) => {
                error!("Invalid syntax detected in config");
                panic!("{e}");
            },
        };

        // allow ~ and $HOME in paths in config.toml
        config.general.music_dir = config.general.music_dir.replacen('~', &home_dir_str, 1);
        config.general.playlist = config.general.playlist.replacen('~', &home_dir_str, 1);
        config.general.music_dir = config.general.music_dir.replacen("$HOME", &home_dir_str, 1);
        config.general.playlist = config.general.playlist.replacen("$HOME", &home_dir_str, 1);

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
            error!(
                "Failed to copy default config from {default_config_path:?} to {config_path:?}: {e}"
            );
            warn!("Did you run make install?")
        }
    }
}
