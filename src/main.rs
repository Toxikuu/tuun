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

mod config;
mod hotkeys;
mod integrations;
mod macros;
mod mpv;
mod structs;
mod traits;

pub static CONFIG: LazyLock<Config> = LazyLock::new(Config::load);
pub static RPC_CLIENT: LazyLock<Mutex<DiscordIpcClient>> = LazyLock::new(|| Mutex::new(DiscordIpcClient::new(&CONFIG.discord.client_id).expect("Invalid discord client id")));
pub static SCROBBLER: Lazy<Mutex<Option<Scrobbler>>> = Lazy::new(|| Mutex::new(None));

#[tokio::main]
async fn main() -> Result<()> {

    // create lock
    if let Err(e) = fs::create_dir("/tmp/tuun")
        .permit(|e| e.kind() == IOE::AlreadyExists)
    {
        eprintln!("Failed to create /tmp/tuun: {e}")
    }

    if let Err(e) = fs::write("/tmp/tuun/tuun.lock", b"") {
        eprintln!("Failed to write to tuun.lock: {e}")
    }

    for a in 1..=8 {
        if RPC_CLIENT.lock().unwrap().connect().is_ok() {
            break
        } else {
            eprintln!("Retrying Discord IPC connection ({a}/8)")
        }
    }

    // authenticate scrobbler in the background
    tokio::spawn(async {
        if let Err(e) = integrations::authenticate_scrobbler().await {
            eprintln!("Error during scrobbler authentication: {e:#?}");
        }
    });

    if should_start_tuunfm() {
        start_process("tuunfm").await;
    }

    tokio::spawn(async {
        mpv::launch().await;
        mpv::connect().await.unwrap();
    });

    register_global_hotkey_handler().await;

    Ok(())
}

fn is_process_running(process: &str) -> bool {
    Command::new("pgrep")
        .args(["-x", process])
        .stdout(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

async fn start_process(process: &str) {
    if !is_process_running(process) {
        if let Err(e) = Command::new(process).spawn() {
            eprintln!("Failed to start {process}: {e}");
        }
    } else {
        eprintln!("{process} is already running")
    }
}

fn should_start_tuunfm() -> bool {
    let home = env::var("HOME").expect("HOME environment variable not set");
    let config_path = format!("{home}/.config/tuun/config.toml");
    let config_content = fs::read_to_string(&config_path).expect("Missing config at ~/.config/tuun/config.toml");

    let parsed = config_content.parse::<toml::Value>().expect("Invalid config");

    if let Some(tfm) = parsed.get("tuunfm") {
        return tfm.get("used").and_then(|v| v.as_bool()).unwrap_or(false)
    }
    false
}
