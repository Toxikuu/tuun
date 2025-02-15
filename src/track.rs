// track.rs
//
// defines the track struct

use crate::mpv::{get_loop_status, get_pause_status, LoopStatus, PauseStatus};
use serde_json::Value;
use std::io::{self, Write};

#[derive(Debug, Clone)]
pub struct Track {
    pub album: String,
    pub artist: String,
    pub arturl: String,
    pub date: String,
    pub duration: f64,
    pub progress: f64,
    pub title: String,
}

impl Track {
    pub fn new(metadata: Value, progress: f64, duration: f64) -> Self {
        let data = metadata.get("data").unwrap().as_object().unwrap();
        let data: serde_json::Map<String, Value> = data
            .iter()
            .map(|(k, v)| (k.to_lowercase(), v.clone()))
            .collect();

        Track {
            // TOOD: I'm confident these fields could be populated more succinctly
            title:  data.get("title") .and_then(|v| v.as_str()).unwrap_or("<Unknown title>").to_string(),
            artist: data.get("artist").and_then(|v| v.as_str()).unwrap_or("<Unknown artist>").to_string(),
            album:  data.get("album") .and_then(|v| v.as_str()).unwrap_or("<Unknown album>").to_string(),
            date:   data.get("date")  .and_then(|v| v.as_str()).unwrap_or("<Unknown release date>").to_string(),
            arturl: data.get("arturl").and_then(|v| v.as_str()).unwrap_or("https://i1.sndcdn.com/artworks-000412100175-y1xaip-t500x500.jpg").to_string(), // cute cat picture when in doubt
            progress,
            duration,
        }
    }

    pub fn display(&self) {
        print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
        print!(
"\
\x1b[36;1mMETADATA\x1b[0m

\x1b[36;1m01 \x1b[30m//// Ttl - \x1b[37m{}\x1b[0m
\x1b[36;1m02 \x1b[30m//// Art - \x1b[37m{}\x1b[0m
\x1b[36;1m03 \x1b[30m//// Alb - \x1b[37m{}\x1b[0m
\x1b[36;1m04 \x1b[30m//// Dte - \x1b[37m{}\x1b[0m
\x1b[36;1m05 \x1b[30m//// Prg - \x1b[37m{:.3}/{:.3}\x1b[0m \
"                                            
, self.title, self.artist, self.album, self.date, self.progress, self.duration
        );

        io::stdout().flush().expect("Failed to flush stdout");
    }

    pub fn is_paused(&self) -> Option<bool> {
        if let Some(status) = get_pause_status() {
            match status {
                PauseStatus::Playing => return Some(false),
                PauseStatus::Paused => return Some(true),
            }
        }
        None
    }

    pub fn is_looped(&self) -> Option<bool> {
        if let Some(status) = get_loop_status() {
            match status {
                LoopStatus::Inf => return Some(true),
                LoopStatus::Not => return Some(false),
            }
        }
        None
    }
}
