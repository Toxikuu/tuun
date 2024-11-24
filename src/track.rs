// track.rs
//
// defines the track struct

use serde_json::Value;
use std::io::{self, Write};

#[derive(Debug, Clone)]
pub struct Track {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub date: String,
    pub arturl: String,
    pub progress: f64,
    pub duration: f64,
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
            title:  data.get("title") .and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
            artist: data.get("artist").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
            album:  data.get("album") .and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
            date:   data.get("date")  .and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
            arturl: data.get("arturl").and_then(|v| v.as_str()).unwrap_or("https://i1.sndcdn.com/artworks-000412100175-y1xaip-t500x500.jpg").to_string(),
            progress,
            duration,
        }
    }

    pub fn display(&self) {
        print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
        print!(
"\
\x1b[36;1mMETADATA\x1b[0m

\x1b[36;1m01 \x1b[30m::: Ttl - \x1b[37m{}\x1b[0m
\x1b[36;1m02 \x1b[30m::: Art - \x1b[37m{}\x1b[0m
\x1b[36;1m03 \x1b[30m::: Alb - \x1b[37m{}\x1b[0m
\x1b[36;1m04 \x1b[30m::: Dte - \x1b[37m{}\x1b[0m
\x1b[36;1m05 \x1b[30m::: Prg - \x1b[37m{:.6}/{:.6}\x1b[0m \
"                                            
, self.title, self.artist, self.album, self. date, self.progress, self.duration
        );

        io::stdout().flush().expect("Failed to flush stdout");
    }
}
