#![deny(clippy::unwrap_used)]

use std::{
    env,
    fs,
    io::ErrorKind as IOE,
    path::PathBuf,
    process::{
        Command,
        Stdio,
        exit,
    },
    sync::LazyLock,
};

use anyhow::Result;
use config::Config;
use discord_rich_presence::DiscordIpcClient;
use hotkeys::register_global_hotkey_handler;
use integrations::connect_discord_rpc_client;
use once_cell::sync::Lazy;
use rustfm_scrobble::Scrobbler;
use tokio::sync::Mutex;
use tracing::{
    debug,
    error,
    info,
    warn,
};
use tracing_appender::rolling;
use tracing_subscriber::{
    EnvFilter,
    fmt,
};
use traits::Permit;

mod config;
mod hotkeys;
mod integrations;
mod mpv;
mod playlists;
mod structs;
mod traits;

pub static CONFIG: LazyLock<Config> = LazyLock::new(Config::load);
pub static RPC_CLIENT: LazyLock<Mutex<DiscordIpcClient>> =
    LazyLock::new(|| Mutex::new(DiscordIpcClient::new(&CONFIG.discord.client_id)));
pub static SCROBBLER: Lazy<Mutex<Option<Scrobbler>>> = Lazy::new(|| Mutex::new(None));

#[tokio::main]
async fn main() -> Result<()> {
    // set up logging
    let file_appender = rolling::never("/tmp/tuun", "log");
    let (file_writer, _guard) = tracing_appender::non_blocking(file_appender);

    let log_level = env::var("TUUN_LOG_LEVEL").unwrap_or("info".to_string());
    let filter = EnvFilter::new(format!("{log_level},winit=info,calloop=info,polling=info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_level(true)
        .with_target(true)
        .with_timer(fmt::time::uptime())
        .with_writer(file_writer)
        .with_line_number(true)
        .compact()
        .init();

    info!("Starting tuun");

    // create /tmp/tuun
    if let Err(e) = fs::create_dir("/tmp/tuun").permit(|e| e.kind() == IOE::AlreadyExists) {
        error!("Failed to create /tmp/tuun: {e}");
        exit(1)
    }

    // create lock
    if let Err(e) = fs::write("/tmp/tuun/tuun.lock", b"") {
        error!("Failed to write to tuun.lock: {e}");
        exit(1)
    }

    info!("Created lock");

    let music_dir = &CONFIG.general.music_dir;
    if !PathBuf::from(music_dir).exists() {
        error!("Music directory '{music_dir}' does not exist!");
        exit(1)
    }

    // create auto-generated playlists
    playlists::create_all_playlist();
    playlists::create_recent_playlist();

    // connect to discord if it's used
    if CONFIG.discord.used {
        connect_discord_rpc_client().await;
    }

    // authenticate lastfm scrobbler in the background if it's used
    if CONFIG.lastfm.used {
        tokio::spawn(async {
            if let Err(e) = integrations::authenticate_lastfm_scrobbler().await {
                error!("Error during scrobbler authentication: {e:#?}");
            } else {
                info!("Authenticated with lastfm");
            }
        });
    }

    // launch tuunfm if it's used
    if CONFIG.tuunfm.used {
        start_process("tuunfm").await;
        info!("Started tuunfm");
    }

    // launch mpv
    tokio::spawn(async {
        info!("Launching MPV");
        mpv::launch().await;
        if let Err(e) = mpv::connect().await {
            error!("Failed to connect to MPV's socket: {e:#?}")
        }
    });

    // register hotkey handler
    register_global_hotkey_handler().await;
    info!("Registered global hotkey handler");

    Ok(())
}

/// Utility function to check if a process is running
/// Returns false on failures
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

/// Utility function to start a process
/// Won't start the process if it's already running
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
