use anyhow::Result;
use config::Config;
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use hotkeys::register_global_hotkey_handler;
use once_cell::sync::Lazy;
use std::{
    env,
    fs,
    io::ErrorKind as IOE,
    process::{Command, Stdio},
    sync::{LazyLock, Mutex}
};
use rustfm_scrobble::Scrobbler;
use traits::Permit;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_appender::rolling;

mod config;
mod hotkeys;
mod integrations;
mod mpv;
mod playlists;
mod structs;
mod traits;

pub static CONFIG: LazyLock<Config> = LazyLock::new(Config::load);
pub static RPC_CLIENT: LazyLock<Mutex<DiscordIpcClient>> = LazyLock::new(|| Mutex::new(DiscordIpcClient::new(&CONFIG.discord.client_id).expect("Invalid discord client id")));
pub static SCROBBLER: Lazy<Mutex<Option<Scrobbler>>> = Lazy::new(|| Mutex::new(None));

#[tokio::main]
async fn main() -> Result<()> {
    // set up logging
    let file_appender = rolling::never("/tmp/tuun", "log");
    let (file_writer, _guard) = tracing_appender::non_blocking(file_appender);

    let log_level = env::var("TUUN_LOG_LEVEL").unwrap_or("info".to_string());
    let filter = EnvFilter::new(format!("{log_level},winit=info,calloop=info,polling=info"));
    
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::Layer::new().with_writer(file_writer))
        .with(fmt::Layer::new().with_writer(std::io::stdout))
        .init();

    info!("Starting tuun");

    // create lock
    if let Err(e) = fs::create_dir("/tmp/tuun")
        .permit(|e| e.kind() == IOE::AlreadyExists)
    {
        error!("Failed to create /tmp/tuun: {e}")
    }

    if let Err(e) = fs::write("/tmp/tuun/tuun.lock", b"") {
        error!("Failed to write to tuun.lock: {e}")
    }
    info!("Created lock");

    playlists::create_all_playlist();
    info!("Created the all playlist");

    for a in 1..=8 {
        if RPC_CLIENT.lock().unwrap().connect().is_ok() {
            info!("Connected to Discord IPC on attempt {a}");
            break
        } else {
            warn!("Retrying Discord IPC connection ({a}/8)")
        }
    }

    // authenticate scrobbler in the background
    if CONFIG.lastfm.used {
        tokio::spawn(async {
            if let Err(e) = integrations::authenticate_lastfm_scrobbler().await {
                error!("Error during scrobbler authentication: {e:#?}");
            } else {
                info!("Authenticated with lastfm");
            }
        });
    }

    if should_start_tuunfm() {
        info!("Starting tuunfm...");
        start_process("tuunfm").await;
        info!("Started tuunfm");
    }

    tokio::spawn(async {
        info!("Launching MPV");
        mpv::launch().await;
        mpv::connect().await.unwrap();
    });

    register_global_hotkey_handler().await;
    info!("Registered global hotkey handler");

    Ok(())
}

fn is_process_running(process: &str) -> bool {
    let running = Command::new("pgrep")
        .args(["-x", process])
        .stdout(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    debug!(process, running, "Checked if process is running");
    running
}

async fn start_process(process: &str) {
    if !is_process_running(process) {
        if let Err(e) = Command::new(process).spawn() {
            error!("Failed to start {process}: {e}");
        } else {
            info!("Started {process}");
        }
    } else {
        warn!("{process} is already running")
    }
}

fn should_start_tuunfm() -> bool {
    let home = env::var("HOME").expect("HOME environment variable not set");
    let config_path = format!("{home}/.config/tuun/config.toml");
    info!("Reading config from {config_path}");
    let config_content = fs::read_to_string(&config_path).expect("Missing config at ~/.config/tuun/config.toml");

    let parsed = config_content.parse::<toml::Value>().expect("Invalid config");

    if let Some(tfm) = parsed.get("tuunfm") {
        return tfm.get("used").and_then(|v| v.as_bool()).unwrap_or(false)
    }
    false
}
