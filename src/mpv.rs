use std::{
    fs,
    path::PathBuf,
    process::exit,
    sync::{
        Arc,
        LazyLock,
        atomic::{
            AtomicBool,
            AtomicU64,
            Ordering,
        },
    },
};

use anyhow::Result;
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
    ARGS,
    CONFIG,
    integrations::{
        lastfm_now_playing,
        lastfm_scrobble,
    },
    structs::Track,
};

const SOCK_PATH: &str = "/tmp/tuun/mpvsocket";

pub static LOOPED: AtomicBool = AtomicBool::new(false);
pub static PAUSED: AtomicBool = AtomicBool::new(false);
pub static MUTED: AtomicBool = AtomicBool::new(false);
pub static VOLUME: AtomicU64 = AtomicU64::new(0);

static FRESH: AtomicBool = AtomicBool::new(false);
static NOW_PLAYING_SET: AtomicBool = AtomicBool::new(false);

static TRACK: LazyLock<Arc<Mutex<Track>>> =
    LazyLock::new(|| Arc::new(Mutex::new(Track::default())));
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
        r#"{"command": ["observe_property", 7, "volume"]}"#,
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

/// # Description
/// Handles MPV events.
/// Supported events include start-file, end-file, and property-change.
async fn handle_events(json: Value) {
    if let Some(event) = json.get("event").and_then(|v| v.as_str()) {
        match event {
            | "start-file" => {
                debug!("MPV Event: New file started");
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
#[allow(clippy::too_many_lines)]
async fn handle_properties(json: Value) {
    if let Some(property) = json.get("name").and_then(Value::as_str) {
        match property {
            | "filename" => {
                info!("MPV Property: Filename changed");
                debug!("Filename property: {json:#}");
            },
            | "pause" => {
                info!("MPV Property: Pause toggled");
                debug!("Pause property: {json:#}");
                if let Some(paused) = json.get("data").and_then(Value::as_bool) {
                    PAUSED.store(paused, Ordering::Relaxed);
                }
                debug!("PAUSED set to {}", PAUSED.load(Ordering::Relaxed));
            },
            | "metadata" => {
                info!("MPV Property: Metadata changed");
                debug!("Metadata property: {json:#}");

                let mut track = TRACK.lock().await;
                if let Err(e) = track.update_metadata(&json).await {
                    error!("Failed to update metadata: {e:#?}");
                }

                info!("Now playing '{track}'");
                if CONFIG.discord.used {
                    track.rpc().await;
                }
                drop(track);
            },
            | "loop-file" => {
                info!("MPV Property: Loop toggled");
                debug!("Loop property: {json:#}");
                if let Some(looped) = json.get("data").and_then(Value::as_bool) {
                    LOOPED.store(looped, Ordering::Relaxed);
                }
                // account for loop="inf"
                if let Some(looped) = json.get("data").and_then(Value::as_str) {
                    let looped = matches!(looped, "inf");
                    LOOPED.store(looped, Ordering::Relaxed);
                }
                debug!("LOOPED set to {}", LOOPED.load(Ordering::Relaxed));
            },
            | "volume" => {
                info!("MPV: Property: Volume changed");
                debug!("Volume: {json:#}");
                if let Some(vol) = json.get("data").and_then(Value::as_f64) {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    VOLUME.store(vol as u64, Ordering::Relaxed);
                }
                debug!("VOLUME set to {}", VOLUME.load(Ordering::Relaxed));
            },
            | "mute" => {
                info!("MPV Property: Mute toggled");
                debug!("Mute property: {json:#}");
                if let Some(muted) = json.get("data").and_then(Value::as_bool) {
                    MUTED.store(muted, Ordering::Relaxed);
                }
                debug!("MUTED set to {}", MUTED.load(Ordering::Relaxed));
            },
            | "playback-time" => {
                trace!("MPV Property: Playback time changed");
                let mut track = TRACK.lock().await;
                let time = json.get("data").and_then(Value::as_f64).unwrap_or(0.);
                trace!("Time: {time}");

                track.update_progress(time);

                if let Err(e) = queue().await {
                    error!("Failed to refresh queue: {e:#}");
                }

                if time == 0. {
                    FRESH.store(true, Ordering::Relaxed);
                    NOW_PLAYING_SET.store(false, Ordering::Relaxed);
                    debug!("Registered track '{track:#?}' as fresh");
                }

                track.display();

                // Set LastFM now playing if the track has been playing for 5 seconds, or it's more
                // than 5% through.
                if time >= (track.duration * 0.05).min(5.)
                    && !NOW_PLAYING_SET.load(Ordering::Relaxed)
                {
                    NOW_PLAYING_SET.store(true, Ordering::Relaxed);

                    if CONFIG.lastfm.used {
                        info!("Setting LastFM now playing");
                        let track_copy = track.clone();
                        tokio::spawn(async move {
                            if let Err(e) = lastfm_now_playing(track_copy).await {
                                error!("Failed to set LastFM now playing: {e:#?}");
                            }
                        });
                    }
                }

                // Scrobble track if it's more than a configurable percent through.
                if time >= (track.duration * (f64::from(CONFIG.lastfm.scrobble_percent) / 100.))
                    && FRESH.load(Ordering::Relaxed)
                {
                    FRESH.store(false, Ordering::Relaxed);

                    if CONFIG.lastfm.used {
                        // TODO: Implement display for track so the logs look nicer
                        info!("Scrobbling track: {track:#?}");
                        let track_copy = track.clone();
                        drop(track);
                        tokio::spawn(async move {
                            if let Err(e) = lastfm_scrobble(track_copy).await {
                                error!("Failed to scrobble track: {e:#?}");
                            }
                        });
                    }
                }
            },
            | _ => {
                warn!("MPV Property: Received unrecognized property:\n{json:#}");
            },
        }
    }
}

#[instrument]
pub async fn launch() {
    info!("Launching mpv...");
    let to_shuffle: &str =
        if ARGS.shuffle.unwrap_or(CONFIG.general.shuffle) { "yes" } else { "no" };

    let mut mpv = Command::new("mpv")
        .arg(format!("--shuffle={to_shuffle}"))
        .arg("--really-quiet")
        .arg("--geometry=350x350+1400+80")
        .arg("--title=tuun-mpv")
        .arg("--loop-playlist=inf")
        .arg(format!("--input-ipc-server={SOCK_PATH}"))
        .args(prequeue())
        .spawn()
        .expect("Failed to launch mpv");
    let pid = mpv.id();

    // Record tuun-mpv's pid, but don't whine if something goes wrong
    if let Some(i) = pid {
        let _ = fs::write("/tmp/tuun/tuun-mpv.pid", i.to_string());
    }

    for a in 1..=32 {
        sleep(Duration::from_millis(
            CONFIG.general.mpv_socket_poll_timeout,
        ))
        .await;
        debug!("Polling MPV socket {a}/32...");
        if fs::metadata(SOCK_PATH).is_ok() {
            debug!("MPV socket was ok on attempt {a}");
            break;
        }
    }

    if let Ok(optcode) = mpv.try_wait()
        && let Some(code) = optcode
        && !code.success()
    {
        error!("MPV exited with a failure");
        if ARGS.playlist.is_some() {
            error!("This is most likely caused by your playlist referencing inaccessible tracks");
        }
    }

    match queue().await {
        | Ok(queued) => {
            if queued {
                info!("Starting with queued tracks");
                if let Err(e) = send_command(r#"{ "command": ["playlist-next"] }"#).await {
                    error!("Failed to skip track for queue start: {e}");
                }
            }
        },
        | Err(e) => error!("Failed to queue tracks from start: {e}"),
    }
}

#[instrument]
fn prequeue() -> Vec<String> {
    // FIXME: This can probably be written less grossly(?)
    let playlist = &ARGS
        .playlist
        .clone()
        .unwrap_or_else(|| CONFIG.general.playlist.clone());

    debug!("Starting with playlist '{playlist}'");
    if !PathBuf::from(playlist).exists() {
        error!("Playlist '{playlist}' does not exist");
        panic!("Playlist '{playlist}' does not exist");
    }

    let args = if QUEUE.exists() {
        debug!("Queue.tpl exists");
        vec![
            format!("--playlist={}", QUEUE.display()),
            format!("--playlist={playlist}"),
        ]
    } else {
        vec![format!("--playlist={playlist}")]
    };

    debug!("Prequeue args for mpv: {args:#?}");
    args
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
