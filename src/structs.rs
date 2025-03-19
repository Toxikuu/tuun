use serde_json::Value;
use std::{fmt, io::{self, Write}};

use crate::{integrations, mpv::send_command, CONFIG};
use tracing::{instrument, trace, debug, warn, error};

#[derive(Debug, Clone)]
pub struct Track {
    pub arturl: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub date: String,
    pub progress: f64,
    pub duration: f64,
}

impl Default for Track {
    fn default() -> Self {
        Self {
            arturl: String::new(),
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            date: String::new(),
            progress: 0.0,
            duration: 1000.,
        }
    }
}

#[derive(Debug)]
pub struct LastFM <'lfm> {
    pub apikey:   &'lfm str,
    pub secret:   &'lfm str,
    pub username: &'lfm str,
    pub password: &'lfm str,
}

impl LastFM<'_> {
    pub fn new() -> Self {
        Self {
            apikey: &CONFIG.lastfm.apikey,
            secret: &CONFIG.lastfm.secret,
            username: &CONFIG.lastfm.user,
            password: &CONFIG.lastfm.password,
        }
    }
}

impl Track {
    #[instrument(level = "debug", skip(metadata))]
    pub async fn update_metadata(&mut self, metadata: &Value) {
        let Some(data) = metadata.get("data") else {
            warn!("Failed to get metadata from {metadata:#}");
            warn!("Not updating metadata");
            return
        };

        let Some(data) = data.as_object() else {
            warn!("Could not convert {data:#} to json");
            warn!("Not updating metadata");
            return
        };

        let data: serde_json::Map<String, Value> = data
            .iter()
            .map(|(k, v)| (k.to_lowercase(), v.clone()))
            .collect();

        // duration is not technically metadata but i count it as such
        let mut duration = None;
        for a in 0..=7 {
            duration = send_command(r#"{"command": ["get_property", "duration"]}"#).await.unwrap().get("data").and_then(|v| v.as_f64());
            if let Some(dur) = duration {
                debug!("Fetched duration {dur} on attempt {a}");
                break
            }
        }

        let Some(dur) = duration else {
            warn!("Failed to fetch duration");
            warn!("Not updating metadata");
            return
        };

        self.title  = data.get("title") .and_then(|v| v.as_str()).unwrap_or("<Unknown title>").to_string();
        self.artist = data.get("artist").and_then(|v| v.as_str()).unwrap_or("<Unknown artist>").to_string();
        self.album  = data.get("album") .and_then(|v| v.as_str()).unwrap_or("<Unknown album>").to_string();
        self.date   = data.get("date")  .and_then(|v| v.as_str()).unwrap_or("<Unknown release date>").to_string();
        self.arturl = data.get("arturl").and_then(|v| v.as_str()).unwrap_or("https://i1.sndcdn.com/artworks-000412100175-y1xaip-t500x500.jpg").to_string(); // cute cat picture when in doubt
        self.duration = dur;
    }

    pub async fn update_progress(&mut self, progress: f64) {
        self.progress = progress;
        trace!("Updated progress to {progress}");
    }

    #[instrument]
    pub fn display(&self) {
        trace!("Displaying track:\n{self:#?}");
        print!("{esc}[2J{esc}[1;1H{esc}[?251", esc = 27 as char);
        if io::stdout().flush().is_err() { return }
        print!(
"\
\x1b[36;1mTUUN\x1b[0m

\x1b[36;1m01 \x1b[30m::: Ttl - \x1b[37m{}\x1b[0m
\x1b[36;1m02 \x1b[30m::: Art - \x1b[37m{}\x1b[0m
\x1b[36;1m03 \x1b[30m::: Alb - \x1b[37m{}\x1b[0m
\x1b[36;1m04 \x1b[30m::: Dte - \x1b[37m{}\x1b[0m
\x1b[36;1m05 \x1b[30m::: Prg - \x1b[37m{:.3}/{:.3}\x1b[0m", 
        self.title, self.artist, self.album, self.date, self.progress, self.duration);
        
        let _ = io::stdout().flush();
    }

    #[instrument]
    pub async fn rpc(&self) {
        if let Err(e) = integrations::discord_rpc(self.clone()).await {
            error!("Error setting discord rpc: {e:#?}")
        }
    }

    pub fn is_default(&self) -> bool {
        self.progress == 0.    &&
        self.duration == 1000. &&
        self.title    == String::with_capacity(0) &&
        self.artist   == String::with_capacity(0) &&
        self.album    == String::with_capacity(0) &&
        self.arturl   == String::with_capacity(0) &&
        self.date     == String::with_capacity(0)
    }
}

impl fmt::Display for Track {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} - {}", self.artist, self.title)
    }
}
