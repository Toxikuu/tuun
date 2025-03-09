use anyhow::Result;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::exit,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, LazyLock,
    }
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader}, 
    net::UnixStream, 
    process::Command, 
    sync::Mutex,
    time::{sleep, Duration},
};

use crate::{integrations::lastfm_scrobble, structs::Track, CONFIG};

const  SOCK_PATH: &str = "/tmp/mpvsocket";
pub static LOOPED:    AtomicBool = AtomicBool::new(false);
pub static PAUSED:    AtomicBool = AtomicBool::new(false);
static FRESH:         AtomicBool = AtomicBool::new(false);
static TRACK:         Lazy<Arc<Mutex<Track>>> = Lazy::new(|| Arc::new(Mutex::new(Track::default())));
static QUEUE:         LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from(&CONFIG.general.playlist).with_file_name("queue.tpl"));

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
            Ok(json) => {
                handle_events(json).await;
        },
            Err(e) => {
                eprintln!("Failed to parse JSON: {e}");
            }
        }
    }
    
    Ok(())
}

pub async fn send_command(command: &str) -> Result<Value> {
    let stream = UnixStream::connect(SOCK_PATH).await?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    
    writer.write_all(command.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    let mut response = String::new();
    reader.read_line(&mut response).await?;

    let json: Value = serde_json::from_str(&response)?;

    Ok(json)
}

pub fn send_command_blocking(command: &str) -> Result<Value> {
    use std::{
        io::{
            BufReader,
            BufRead,
            Write,
        },
        os::unix::net::UnixStream,
    };

    let mut stream = UnixStream::connect(SOCK_PATH)?;

    stream.write_all(command.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;
    
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response)?;
    
    let json: Value = serde_json::from_str(&response)?;

    Ok(json)
}

/// # Description
/// Handles MPV events.
/// Supported events include start-file, end-file, and property-change.
async fn handle_events(json: Value) {
    if let Some(event) = json.get("event").and_then(|v| v.as_str()) {
        match event {
            "start-file" => {
                println!(" //// NEW SONG STARTED")
            },
            "end-file" => {
                if let Some(reason) = json.get("reason").and_then(|v| v.as_str()) {
                    if reason == "quit" {
                        println!(" //// MPV QUIT");
                        exit(0)
                    } else {
                        println!(" //// EOF WITH REASON: {reason}");
                    }
                }
            },
            "property-change" => {
                handle_properties(json).await;
            }
            _ => {
                // should only be included in verbose mode
                println!("Received event: {event:#}")
            }
        }
    }
} 

/// Handles MPV properties.
/// Supported properties include filename, pause, loop-file, mute, and playback-time
async fn handle_properties(json: Value) {
    if let Some(property) = json.get("name").and_then(|v| v.as_str()) {
        match property {
            "filename" => {
                println!(" //// FILENAME CHANGED");
                println!(" //// Filename property: {json:#}");
            },
            "pause" => {
                println!(" //// PAUSE TOGGLED");
                println!(" //// Pause property: {json:#}");
                if let Some(paused) = json.get("data").and_then(|v| v.as_bool()) {
                    PAUSED.store(paused, Ordering::Relaxed);
                };
            },
            "loop-file" => {
                println!(" //// LOOP TOGGLED");
                println!(" //// Loop property: {json:#}");
                if let Some(looped) = json.get("data").and_then(|v| v.as_bool()) {
                    LOOPED.store(looped, Ordering::Relaxed);
                };
                // account for loop="inf"
                if let Some(looped) = json.get("data").and_then(|v| v.as_str()) {
                    let looped = matches!(looped, "inf");
                    LOOPED.store(looped, Ordering::Relaxed);
                };
            },
            "mute" => {
                println!(" //// MUTE TOGGLED");
                println!(" //// Mute property: {json:#}");
            },
            "playback-time" => {
                let mut track = TRACK.lock().await;
                let time = json.get("data").and_then(|v| v.as_f64()).unwrap_or(0.);

                track.update_progress(time).await;

                if let Err(e) = queue().await {
                    eprintln!("Failed to refresh queue: {e}")
                }

                if time == 0. {
                    FRESH.store(true, Ordering::Relaxed);
                }

                track.display();
                if time >= (track.duration / 4.) && FRESH.load(Ordering::Relaxed) {
                    FRESH.store(false, Ordering::Relaxed);

                    let track_copy = track.clone();
                    tokio::spawn(async move {
                        if let Err(e) = tokio::task::block_in_place(|| {
                            lastfm_scrobble(track_copy)
                        }) {
                            eprintln!("Failed to scrobble track: {e:#?}")
                        }
                    });
                }
            },
            "metadata" => {
                println!(" //// METADATA CHANGED");
                println!(" //// Metadata property: {json:#}");
                let mut track = TRACK.lock().await;
                track.update_metadata(&json).await;
                track.rpc().await;
            },
            _ => {
                println!("Received property: {json:#}")
            }
        }
    }
}

pub async fn launch() {
    Command::new("mpv")
        .arg("--shuffle")
        .arg("--really-quiet")
        .arg("--geometry=350x350+1400+80")
        .arg("--title='tuun-mpv'")
        .arg("--input-ipc-server=/tmp/mpvsocket")
        .args(prequeue())
        .spawn()
        .expect("Failed to launch mpv");

    for a in 1..=32 {
        sleep(Duration::from_millis(128)).await;
        println!("Polling mpv socket {a}/32...");
        if fs::metadata("/tmp/mpvsocket").is_ok() {
            break
        }
    }

    match queue().await {
        Ok(true) => {
            if let Err(e) = send_command(r#"{ "command": ["playlist-next"] }"#).await {
                eprintln!("Failed to skip track for queue start: {e}")
            }
        },
        Ok(false) => {},
        Err(e) => eprintln!("Failed to queue tracks from start: {e}")
    }
}

fn prequeue() -> Vec<String> {
    let playlist = PathBuf::from(&CONFIG.general.playlist);
    let queue = playlist.with_file_name("queue.tpl");
    if queue.exists() {
        // vec![format!("--playlist={}", queue.display()), format!("--playlist={}", playlist.display())]
        vec![format!("--playlist={}", playlist.display())]
    } else {
        vec![format!("--playlist={}", playlist.display())]
    }
}

async fn queue() -> Result<bool> {
    let queue = &*QUEUE;
    if !queue.exists() {
        return Ok(false)
    }

    let songs = fs::read_to_string(queue)?;

    for song in songs.lines() {
        let song = song.trim();
        let command = format!(r#"{{ "command": ["loadfile", "{song}", "insert-next"] }}"#);
        send_command(&command).await?;
        println!("Queued {song}");
    }

    fs::remove_file(queue)?;
    Ok(true)
}
