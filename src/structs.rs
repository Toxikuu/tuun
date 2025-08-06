use std::{
    fmt, io::{
        self,
        Write,
    }, path::PathBuf, sync::atomic::Ordering
};

use std::path::Path;
use id3::{Tag, Content};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use tracing::{
    debug,
    error,
    instrument,
    trace,
    warn,
};

use crate::{
    CONFIG,
    config::ColorConfig,
    integrations,
    mpv::{
        LOOPED,
        MUTED,
        VOLUME,
        send_command,
    },
};

#[derive(Debug, Clone)]
pub struct Track {
    pub arturl:   String,
    pub title:    String,
    pub artist:   String,
    pub album:    String,
    pub date:     String,
    pub progress: f64,
    pub duration: f64,
}

impl Default for Track {
    fn default() -> Self {
        Self {
            arturl:   String::new(),
            title:    String::new(),
            artist:   String::new(),
            album:    String::new(),
            date:     String::new(),
            progress: 0.0,
            duration: 1000.,
        }
    }
}

#[derive(Debug)]
pub struct LastFM<'lfm> {
    pub apikey:   &'lfm str,
    pub secret:   &'lfm str,
    pub username: &'lfm str,
    pub password: &'lfm str,
}

impl LastFM<'_> {
    pub fn new() -> Self {
        Self {
            apikey:   &CONFIG.lastfm.apikey,
            secret:   &CONFIG.lastfm.secret,
            username: &CONFIG.lastfm.user,
            password: &CONFIG.lastfm.password,
        }
    }
}

pub fn strip_null(s: &str) -> String { s.replace('\0', "") }

impl Track {
    pub async fn query_metadata(&self) -> Result<Value> {
        let command = r#" { "command" : [ "get_property", "metadata" ] "#;
        send_command(command).await
    }

    pub async fn query_filename(&self) -> Result<PathBuf> {
        let command = r#" { "command" : [ "get_property", "filename" ] "#;
        let Ok(data) = send_command(command).await else {
            warn!("Failed to query filename");
            bail!("Failed to query filename");
        };

        let Some(data) = data.as_object() else {
            warn!("MPV returned invalid JSON");
            bail!("Invalid JSON");
        };

        let filename = data.get("value").context("Filename not present")?.to_string();
        Ok(PathBuf::from(filename))
    }

    pub async fn get_arturl(&self, data: &serde_json::Map<String, Value>) -> Option<String> {
        if let Some(url) = data.get("arturl").and_then(|v| v.as_str()) {
            return Some(url.to_string())
        }

        // TODO: See if I can get the full filepath from MPV instead of just the filename
        let filename = self.query_filename().await.ok()?;
        let tag = Tag::read_from_path(Path::new(&CONFIG.general.music_dir).join(filename)).ok()?;

        return tag.frames()
            .find_map(|f| match f.content() {
                | Content::ExtendedLink(l) if l.description == "Cover" => Some(l.link.clone()),
                | Content::ExtendedText(t) if t.description == "arturl" => Some(strip_null(&t.value)),
                | _ => None
            })
    }

    #[instrument(level = "debug")]
    pub async fn update_metadata(
        &mut self,
        metadata: &Value,
    ) -> Result<()> {
        let Some(data) = metadata.get("data").and_then(|d| d.as_object()) else {
            warn!("Failed to get metadata from {metadata:#}");
            warn!("Not updating metadata");
            return Ok(())
        };

        let data: serde_json::Map<String, Value> = data
            .iter()
            .map(|(k, v)| (k.to_lowercase(), v.clone()))
            .collect();

        self.title = data.get("title").and_then(|v| v.as_str())
            .unwrap_or("<Unknown title>")
            .to_string();
        self.artist = data.get("artist").and_then(|v| v.as_str())
            .unwrap_or("<Unknown artist>")
            .to_string();
        self.album = data.get("album").and_then(|v| v.as_str())
            .unwrap_or("<Unknown album>")
            .to_string();
        self.date = data.get("date").and_then(|v| v.as_str())
            .unwrap_or("<Unknown date>")
            .to_string();

        self.arturl = self.get_arturl(&data).await
            .unwrap_or_else(|| CONFIG.discord.fallback_art.to_string());

        debug!("Attempting to find duration");
        // duration is not technically metadata but i count it as such
        let mut duration = None;
        for a in 0..u16::MAX {
            duration = send_command(r#"{"command": ["get_property", "duration"]}"#)
                .await?
                .get("data")
                .and_then(|v| v.as_f64());
            if let Some(dur) = duration {
                debug!("Fetched duration {dur} on attempt {a}");
                break;
            }
        }

        let Some(dur) = duration else {
            error!("Failed to fetch duration after {} attempts", u16::MAX);
            warn!("Not updating metadata");
            return Ok(());
        };

        self.duration = dur;

        Ok(())
    }

    pub async fn update_progress(&mut self, progress: f64) {
        self.progress = progress;
        trace!("Updated progress to {progress}");
    }

    fn format_metadata(&self) -> String {
        let loop_display = if LOOPED.load(Ordering::Relaxed) { " (looped)" } else { "" };
        let mute_display = if MUTED.load(Ordering::Relaxed) { " (muted)" } else { "" };

        let theme = Theme::from(&CONFIG.color);

        format!(
            "\
{b}{p}TUUN {ver}{r}

{b}{p}01 {s}{sep} Ttl - {t}{title}{r}
{b}{p}02 {s}{sep} Art - {t}{artist}{r}
{b}{p}03 {s}{sep} Alb - {t}{album}{r}
{b}{p}04 {s}{sep} Dte - {t}{date}{r}
{b}{p}05 {s}{sep} Vol - {t}{volume}{muted}{r}
{b}{p}06 {s}{sep} Prg - {t}{progress:.3}/{duration:.3}{looped}{r}",
            p = theme.p,
            s = theme.s,
            t = theme.t,
            r = "\x1b[0m",
            b = "\x1b[1m",
            sep = ":::",
            ver = env!("CARGO_PKG_VERSION"),
            title = self.title,
            artist = self.artist,
            album = self.album,
            date = self.date,
            volume = VOLUME.load(Ordering::Relaxed),
            muted = mute_display,
            progress = self.progress,
            duration = self.duration,
            looped = loop_display,
        )
    }

    #[instrument]
    pub fn display(&self) {
        trace!("Displaying track:\n{self:#?}");
        if let Err(e) = cls() {
            warn!("Failed to cls: {e}");
            return
        }

        let out = self.format_metadata();
        print!("{out}");

        if let Err(e) = io::stdout().flush() {
            warn!("Failed to print metadata: {e:#?}");
        }
    }

    #[instrument]
    pub async fn rpc(&self) {
        if let Err(e) = integrations::discord_rpc(self.clone()).await {
            error!("Error setting discord rpc: {e:#?}")
        }
    }

    #[rustfmt::skip]
    pub fn is_default(&self) -> bool {
        self.progress == 0.
            && self.duration == 1000.
            && self.title    == String::with_capacity(0)
            && self.artist   == String::with_capacity(0)
            && self.album    == String::with_capacity(0)
            && self.arturl   == String::with_capacity(0)
            && self.date     == String::with_capacity(0)
    }
}

impl fmt::Display for Track {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} - {}", self.artist, self.title)
    }
}

fn hex_to_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.trim_start_matches('#');
    match hex.len() {
        | 3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            Some((r, g, b))
        },
        | 6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some((r, g, b))
        },
        | _ => None,
    }
}

fn fg_rgb(r: u8, g: u8, b: u8) -> String { format!("\x1b[38;2;{r};{g};{b}m") }

fn fg_hex(hex: &str) -> String {
    if let Some((r, g, b)) = hex_to_rgb(hex) {
        fg_rgb(r, g, b)
    } else {
        String::new()
    }
}

#[derive(Debug)]
pub struct Theme {
    /// primary
    pub p: String,
    /// secondary
    pub s: String,
    /// tertiary
    pub t: String,
}

impl From<&ColorConfig> for Theme {
    fn from(cfg: &ColorConfig) -> Self {
        Self {
            p: fg_hex(&cfg.primary),
            s: fg_hex(&cfg.secondary),
            t: fg_hex(&cfg.tertiary),
        }
    }
}

fn cls() -> io::Result<()> {
    print!("{esc}[2J{esc}[1;1H{esc}[?251", esc = 27 as char);
    io::stdout().flush()
}
