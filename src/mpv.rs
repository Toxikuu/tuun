// mpv.rs
//
// responsible for interacting with mpv

use crate::track::Track;
use crate::{vpr, erm};
use serde_json::Value;
use std::process::Command;
use std::io::{Write, BufReader, BufRead};
use std::os::unix::net::UnixStream;
use std::time::Duration;
use std::thread::sleep;
use std::fs;

const PLAYLIST: &str ="/home/t/Music/Playlists/all.tpl";
const SOCKET: &str = "/tmp/mpvsocket";
const RETRY_DELAY: u64 = 12;

pub fn wait_for_socket() {
    sleep(Duration::from_millis(20)); // initial wait
    if fs::metadata(SOCKET).is_err() {
        panic!("Timed out waiting for mpv socket to be ready");
    }
}

pub fn launch_mpv() {
    let child = Command::new("mpv")
        .arg("--shuffle")
        .arg("--really-quiet")
        .arg("--geometry=350x350+1400+80")
        .arg("--title='tuun-mpv'")
        .arg(format!("--playlist={}", PLAYLIST))
        .arg(format!("--input-ipc-server={}", SOCKET))
        .spawn()
        .expect("Failed to launch mpv");

    wait_for_socket();
    vpr!("mpv launched with PID: {}", child.id());
}

pub fn mpv_cmd(command: &str) -> Result<String, String> {
    vpr!("Sending mpv command '{}'", command);
    let command = format!("{}\n", command);

    let mut stream = UnixStream::connect(SOCKET)
        .map_err(|e| format!("Failed to connect to mpv ipc: {}", e))?;

    stream.set_read_timeout(Some(Duration::from_millis(50)))
        .map_err(|e| format!("Failed to set timeout: {}", e))?;

    stream.write_all(command.as_bytes())
        .map_err(|e| format!("Failed to send command: {}", e))?;

    let mut reader = BufReader::new(stream);
    let mut response = String::new();

    let start = std::time::Instant::now();
    let timeout = Duration::from_millis(60);

    loop {
        if start.elapsed() > timeout {
            return Err("Command timed out".to_string());
        }
        
        response.clear();
        match reader.read_line(&mut response) {
            Ok(0) => return Err("EOF while reading response".to_string()),
            Ok(_) => {
                if response.contains(r#""data""#) {
                    return Ok(response);
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock 
                   || e.kind() == std::io::ErrorKind::TimedOut => {
                continue;
            },
            Err(e) => return Err(format!("Failed to read response: {}", e)),
        };
    }
}


pub fn get_progress() -> Option<f64> {
    vpr!("Getting progress...");
    
    let command = r#"{ "command": ["get_property", "playback-time"] }"#;
    for _ in 1..=15 {
        match mpv_cmd(command) {
            Ok(r) => {
                if let Ok(v) = serde_json::from_str::<Value>(&r) {
                    if let Some(progress) = v.get("data").and_then(|d| d.as_f64()) {
                        return Some(progress);
                    }
                    erm!("Error getting progress: Missing data field in json!");
                    sleep(Duration::from_millis(RETRY_DELAY));
                    vpr!("Continuing...");
                    continue
                }
                erm!("Error getting progress: Failed to convert r to json!");
                sleep(Duration::from_millis(RETRY_DELAY));
                vpr!("Continuing...");
                continue
            }
            Err(_e) => {
                erm!("Error getting progress: Unknown error!");
                sleep(Duration::from_millis(RETRY_DELAY));
                vpr!("Continuing...");
                continue
            }
        }
    }
    None
}


pub fn get_duration() -> Option<f64> {
    vpr!("Getting duration...");

    let command = r#"{ "command": ["get_property", "duration"] }"#;
    for _ in 1..=15 {
        match mpv_cmd(command) {
            Ok(r) => {
                if let Ok(v) = serde_json::from_str::<Value>(&r) {
                    if let Some(duration) = v.get("data").and_then(|d| d.as_f64()) {
                        return Some(duration);
                    }
                    erm!("Error getting duration: Missing data field in json!");
                    sleep(Duration::from_millis(RETRY_DELAY));
                    vpr!("Continuing...");
                    continue
                }
                erm!("Error getting duration: Error converting r to json!");
                sleep(Duration::from_millis(RETRY_DELAY));
                vpr!("Continuing...");
                continue
            }
            Err(_e) => {
                erm!("Error getting duration: Unknown error!");
                sleep(Duration::from_millis(RETRY_DELAY));
                vpr!("Continuing...");
                continue
            }
        }
    }
    None
}


pub fn get_metadata() -> Value {
    vpr!("Getting metadata...");

    let command = r#"{ "command": ["get_property", "metadata"] }"#;
    for _a in 1..=15 {
        vpr!("Metadata fetch attempt {}", _a);

        match mpv_cmd(command) {
            Ok(r) => {
                if let Ok(v) = serde_json::from_str::<Value>(&r) {
                    if v.get("data").is_some() {
                        return v
                    }
                    erm!("Error getting metadata: Missing data field in json!");
                    sleep(Duration::from_millis(RETRY_DELAY));
                    vpr!("Continuing...");
                    continue
                }
                erm!("Error getting metadata: Error converting r to json!");
                sleep(Duration::from_millis(RETRY_DELAY));
                vpr!("Continuing...");
                continue
            }
            Err(_e) => {
                erm!("Error getting metadata: Unknown error!");
                sleep(Duration::from_millis(RETRY_DELAY));
                vpr!("Continuing...");
                continue
            }
        }
    }
    panic!()
}

pub fn form_track() -> Track {
    let metadata = get_metadata();
    let progress = get_progress().unwrap_or(0.);
    let duration = get_duration().unwrap_or(0.);
    Track::new(metadata, progress, duration)
}
