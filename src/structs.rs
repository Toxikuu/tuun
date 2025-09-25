use std::{
    fmt,
    io::{
        self,
        Write,
    },
    path::PathBuf,
    sync::atomic::Ordering,
};

use anyhow::{
    Context,
    Result,
    bail,
};
use id3::{
    Content,
    Tag, TagLike,
};
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
use urlencoding::encode;

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

pub fn urlencode_arturl(arturl: &str) -> String {
    let (proto, rest_of_url) = arturl.find("://")
        .map_or(("", arturl), |index| (&arturl[..index + 3], &arturl[index + 3..]));
    let encoded_rest = rest_of_url
        .split('/')
        .map(|part| encode(part).into_owned())
        .collect::<Vec<_>>()
        .join("/");
    format!("{proto}{encoded_rest}")
}

impl Track {
    // TODO: See if this should be used anywhere
    #[allow(unused)]
    pub async fn query_metadata(&self) -> Result<Value> {
        let command = r#" { "command" : [ "get_property", "metadata" ] "#;
        send_command(command).await
    }

    pub async fn query_filepath(&self) -> Result<PathBuf> {
        let command = r#" { "command" : [ "get_property", "path" ] } "#;
        let Ok(data) = send_command(command).await else {
            warn!("Failed to query path");
            bail!("Failed to query path");
        };

        let Some(data) = data.as_object() else {
            warn!("MPV returned invalid JSON");
            bail!("Invalid JSON");
        };

        let filename = data
            .get("data")
            .and_then(|v| v.as_str())
            .context("Filename not present")?;
        Ok(PathBuf::from(filename))
    }

    #[instrument(skip(data))]
    pub fn get_arturl(data: &serde_json::Map<String, Value>, tag: Option<&Tag>) -> Option<String> {
        if let Some(url) = data.get("arturl").and_then(|v| v.as_str()) {
            debug!("Using key 'arturl' from mpv's metadata");
            return Some(url.to_string());
        }

        if let Some(tag) = tag {
            let arturl = tag.frames().find_map(|f| match f.content() {
                | Content::ExtendedLink(l) if l.description == "Cover" => Some(l.link.clone()),
                | Content::ExtendedText(t) if t.description == "arturl" => Some(strip_null(&t.value)),
                | _ => {
                    warn!("Couldn't find arturl in extended text or cover in extended link frames");
                    debug!("Frames: {f:#?}");
                    None
                },
            });

            return arturl;
        }

        None
    }

    #[instrument(skip(data))]
    pub fn get_artists(data: &serde_json::Map<String, Value>, tag: Option<&Tag>) -> String {
        if let Some(tag) = tag {
            let artist = tag.artist();
            match artist {
                Some(a) => return a.split('\0').collect::<Vec<_>>().join(", "),
                None => return String::from("<Unknown artist>"),
            }
        }

        data.get("artist")
            .and_then(|v| v.as_str())
            .unwrap_or("<Unknown artist>")
            .to_string()
    }

    #[instrument(level = "debug")]
    pub async fn update_metadata(&mut self, metadata: &Value) -> Result<()> {
        let Some(data) = metadata.get("data").and_then(|d| d.as_object()) else {
            warn!("Failed to get metadata from {metadata:#}");
            warn!("Not updating metadata");
            return Ok(());
        };

        let data: serde_json::Map<String, Value> = data
            .iter()
            .map(|(k, v)| (k.to_lowercase(), v.clone()))
            .collect();

        let filepath = match self.query_filepath().await {
            | Ok(f) => Some(f),
            | Err(e) => {
                warn!("Couldn't get filepath: {e}");
                warn!("Metadata might be less accurate");
                None
            },
        };

        let tag = if let Some(f) = filepath
            && let Some(ext) = f.extension()
            && ext.eq_ignore_ascii_case("mp3") {
            match Tag::read_from_path(&f) {
                | Ok(t) => Some(t),
                | Err(e) => {
                    error!("Couldn't read tag from path '{}': {e}", f.display());
                    None
                },
            }
        } else { None };

        self.title = data
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("<Unknown title>")
            .to_string();
        self.artist = Self::get_artists(&data, tag.as_ref());
        self.album = data
            .get("album")
            .and_then(|v| v.as_str())
            .unwrap_or("<Unknown album>")
            .to_string();
        self.date = data
            .get("date")
            .and_then(|v| v.as_str())
            .unwrap_or("<Unknown date>")
            .to_string();

        self.arturl = urlencode_arturl(
            &Self::get_arturl(&data, tag.as_ref())
            .unwrap_or_else(|| CONFIG.discord.fallback_art.to_string())
        );

        debug!("Attempting to find duration");
        // duration is not technically metadata but i count it as such
        let mut duration = None;
        for a in 0..u16::MAX {
            duration = send_command(r#"{"command": ["get_property", "duration"]}"#)
                .await?
                .get("data")
                .and_then(Value::as_f64);
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

    pub fn update_progress(&mut self, progress: f64) {
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
            return;
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
            error!("Error setting discord rpc: {e:#?}");
        }
    }

    #[rustfmt::skip]
    #[allow(clippy::float_cmp)] // handled with .round()
    pub fn is_default(&self) -> bool {
        self.progress == 0.
            && self.duration.round() == 1000.0
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
