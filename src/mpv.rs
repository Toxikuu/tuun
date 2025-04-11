use std::{
    fs,
    path::PathBuf,
    process::exit,
    sync::{
        Arc,
        LazyLock,
        atomic::{
            AtomicBool,
            Ordering,
        },
    },
};

use anyhow::Result;
use once_cell::sync::Lazy;
use serde_json::Value;
use tokio::{
    io::{
        AsyncBufReadExt,
        AsyncWriteExt,
        BufReader,
    },
    net::UnixStream,
    process::Command,
    sync::Mutex,
    time::{
        Duration,
        sleep,
    },
};
use tracing::{
    debug,
    error,
    info,
    instrument,
    trace,
    warn,
};

use crate::{
    CONFIG,
    integrations::lastfm_scrobble,
    structs::Track,
};

const SOCK_PATH: &str = "/tmp/tuun/mpvsocket";
pub static LOOPED: AtomicBool = AtomicBool::new(false);
pub static PAUSED: AtomicBool = AtomicBool::new(false);
static FRESH: AtomicBool = AtomicBool::new(false);
static TRACK: Lazy<Arc<Mutex<Track>>> = Lazy::new(|| Arc::new(Mutex::new(Track::default())));
static QUEUE: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("/tmp/tuun/quu.tpl"));

pub async fn connect() -> Result<()> {
    // Connect to mpv's socket
    let stream = UnixStream::connect(SOCK_PATH).await?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // The second parameter is an arbitrary observation ID
    // To find more properties, press 'gr' while hovering over mpv
    let subscriptions = [
        r#"{"command": ["observe_property", 1, "filename"]}"#,
        r#"{"command": ["observe_property", 2, "pause"]}"#,
        r#"{"command": ["observe_property", 3, "loop-file"]}"#,
        r#"{"command": ["observe_property", 4, "mute"]}"#,
        r#"{"command": ["observe_property", 5, "playback-time"]}"#,
        r#"{"command": ["observe_property", 6, "metadata"]}"#,
    ];

    // Send all subscription commands
    for command in &subscriptions {
        writer.write_all(command.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }

    // Continuously read lines from mpv's socket
    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            // EOF
            break;
        }

        match serde_json::from_str::<Value>(&line) {
            | Ok(json) => {
                handle_events(json).await;
            },
            | Err(e) => {
                eprintln!("Failed to parse JSON: {e}");
            },
        }
    }

    Ok(())
}

#[instrument(level = "debug")]
pub async fn send_command(command: &str) -> Result<Value> {
    let stream = UnixStream::connect(SOCK_PATH).await?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    debug!("Connected to mpv socket {SOCK_PATH:?}");

    writer.write_all(command.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;
    debug!("Sent mpv command: {command:#}");

    let mut response = String::new();
    reader.read_line(&mut response).await?;

    let json: Value = serde_json::from_str(&response)?;
    debug!("Received mpv response: {json:#}");

    Ok(json)
}

#[instrument(level = "debug")]
pub fn send_command_blocking(command: &str) -> Result<Value> {
    use std::{
        io::{
            BufRead,
            BufReader,
            Write,
        },
        os::unix::net::UnixStream,
    };

    let mut stream = UnixStream::connect(SOCK_PATH)?;

    stream.write_all(command.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;
    debug!("Sending blocking mpv command: {command:#}");

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response)?;

    let json: Value = serde_json::from_str(&response)?;
    debug!("Received blocking mpv command response: {json:#}");

    Ok(json)
}

/// # Description
/// Handles MPV events.
/// Supported events include start-file, end-file, and property-change.
async fn handle_events(json: Value) {
    if let Some(event) = json.get("event").and_then(|v| v.as_str()) {
        match event {
            | "start-file" => {
                debug!("MPV Event: New file started")
            },
            | "end-file" => {
                if let Some(reason) = json.get("reason").and_then(|v| v.as_str()) {
                    if reason == "quit" {
                        info!("MPV quit. Exiting...");
                        exit(0)
                    } else {
                        debug!("MPV Event: EOF:\n{reason:#}");
                    }
                }
            },
            | "property-change" => {
                trace!("Detected property change: {json:#}");
                handle_properties(json).await;
            },
            | _ => {
                trace!("MPV Event: Received uncategorized event:\n{event:#}");
            },
        }
    }
}

/// Handles MPV properties.
/// Supported properties include filename, pause, loop-file, mute, and playback-time
#[instrument(level = "trace")]
async fn handle_properties(json: Value) {
    if let Some(property) = json.get("name").and_then(|v| v.as_str()) {
        match property {
            | "filename" => {
                debug!("MPV Property: Filename changed");
                debug!("Filename property: {json:#}");
            },
            | "pause" => {
                info!("MPV Property: Pause toggled");
                debug!("Pause property: {json:#}");
                if let Some(paused) = json.get("data").and_then(|v| v.as_bool()) {
                    PAUSED.store(paused, Ordering::Relaxed);
                };
                debug!("PAUSED set to {}", PAUSED.load(Ordering::Relaxed));
            },
            | "loop-file" => {
                info!("MPV Property: Loop toggled");
                debug!("Loop property: {json:#}");
                if let Some(looped) = json.get("data").and_then(|v| v.as_bool()) {
                    LOOPED.store(looped, Ordering::Relaxed);
                };
                // account for loop="inf"
                if let Some(looped) = json.get("data").and_then(|v| v.as_str()) {
                    let looped = matches!(looped, "inf");
                    LOOPED.store(looped, Ordering::Relaxed);
                };
                debug!("LOOPED set to {}", LOOPED.load(Ordering::Relaxed));
            },
            | "mute" => {
                info!("MPV Property: Mute toggled");
                debug!("Mute property: {json:#}");
            },
            | "playback-time" => {
                trace!("MPV Property: Playback time changed");
                let mut track = TRACK.lock().await;
                let time = json.get("data").and_then(|v| v.as_f64()).unwrap_or(0.);
                trace!("Time: {time}");

                track.update_progress(time).await;

                if let Err(e) = queue().await {
                    error!("Failed to refresh queue: {e:#}");
                }

                if time == 0. {
                    FRESH.store(true, Ordering::Relaxed);
                    debug!("Registered track '{track:#?}' as fresh");
                }

                track.display();
                if time >= (track.duration / 4.) && FRESH.load(Ordering::Relaxed) {
                    FRESH.store(false, Ordering::Relaxed);

                    if CONFIG.lastfm.used {
                        // TODO: Implement display for track so the logs look nicer
                        info!("Scrobbling track: {track:#?}");
                        let track_copy = track.clone();
                        tokio::spawn(async move {
                            if let Err(e) = lastfm_scrobble(track_copy).await {
                                error!("Failed to scrobble track: {e:#?}");
                            }
                        });
                    }
                }
            },
            | "metadata" => {
                debug!("MPV Property: Metadata changed");
                debug!("Metadata property: {json:#}");

                let mut track = TRACK.lock().await;
                if let Err(e) = track.update_metadata(&json).await {
                    error!("Failed to update metadata: {e:#?}")
                }
                info!("Now playing '{track}'");
                if CONFIG.discord.used {
                    track.rpc().await;
                }
            },
            | _ => {
                warn!("MPV Property: Received unrecognized property:\n{json:#}")
            },
        }
    }
}

#[instrument]
pub async fn launch() {
    info!("Launching mpv...");
    Command::new("mpv")
        .arg(if CONFIG.general.shuffle { "--shuffle=yes" } else { "--shuffle=no" })
        .arg("--really-quiet")
        .arg("--geometry=350x350+1400+80")
        .arg("--title='tuun-mpv'")
        .arg(format!("--input-ipc-server={SOCK_PATH}"))
        .args(prequeue())
        .spawn()
        .expect("Failed to launch mpv");

    for a in 1..=32 {
        sleep(Duration::from_millis(128)).await;
        debug!("Polling MPV socket {a}/32...");
        if fs::metadata(SOCK_PATH).is_ok() {
            debug!("MPV socket was ok on attempt {a}");
            break;
        }
    }

    match queue().await {
        | Ok(queued) => {
            if queued {
                info!("Starting with queued tracks");
                if let Err(e) = send_command(r#"{ "command": ["playlist-next"] }"#).await {
                    error!("Failed to skip track for queue start: {e}")
                }
            }
        },
        | Err(e) => error!("Failed to queue tracks from start: {e}"),
    }
}

#[instrument]
fn prequeue() -> Vec<String> {
    let playlist = &CONFIG.general.playlist;
    if QUEUE.exists() {
        vec![
            format!("--playlist={}", QUEUE.display()),
            format!("--playlist={playlist}"),
        ]
    } else {
        vec![format!("--playlist={playlist}")]
    }
}

#[instrument]
async fn queue() -> Result<bool> {
    let queue = &*QUEUE;

    trace!("Checking whether queue {queue:?} exists...");
    if !queue.exists() {
        trace!("No songs queued");
        return Ok(false);
    }

    let songs = fs::read_to_string(queue)?;

    for song in songs.lines() {
        let song = song.trim();
        let command = format!(r#"{{ "command": ["loadfile", "{song}", "insert-next"] }}"#);
        send_command(&command).await?;
        info!("Queued {song}");
    }

    fs::remove_file(queue)?;
    debug!("Removed queue file {queue:?}");
    Ok(true)
}
