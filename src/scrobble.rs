// scrobble.rs
//
// responsible for scrobbling

use crate::globals::CONFIG;
use rustfm_scrobble::{Scrobble, Scrobbler};
use crate::track::Track;
use chrono::Utc;
use serde_json::json;
use crate::{vpr, erm};

#[derive(Debug, Clone)]
pub struct LastFM {
    pub apikey: String,
    pub secret: String,
    pub username: String,
    pub password: String,
}

impl LastFM {
    pub fn new(apikey: &str, secret: &str, username: &str, password: &str) -> Self {
        Self { 
            apikey: apikey.to_string(),
            secret: secret.to_string(),
            username: username.to_string(),
            password: password.to_string()
        }
    }
}

pub fn lfm_scrobble(track: &Track) {
    let lfm = LastFM::new(
        &CONFIG.lastfm.apikey,
        &CONFIG.lastfm.secret,
        &CONFIG.lastfm.user,
        &CONFIG.lastfm.password,
    );

    let mut scrobbler = Scrobbler::new(&lfm.apikey, &lfm.secret);

    if let Err(e) = scrobbler.authenticate_with_password(&lfm.username, &lfm.password) {
        vpr!("Failed to authenticate with LastFM: {:#?}", e); 
        return;
    }
    vpr!("Authenticated!");

    let track = Scrobble::new(&track.artist, &track.title, &track.album);
    if let Err(e) = scrobbler.now_playing(&track) {
        vpr!("Failed to set track as now playing with LastFM: {:#?}", e);
        return;
    }
    vpr!("Sent now playing!");

    if let Err(e) = scrobbler.scrobble(&track) {
        vpr!("Failed to scrobble track with LastFM: {:?}", e);
        return;
    }
    vpr!("Sent scrobble!");
}

pub fn tfm_scrobble(track: &Track) {
    let timestamp = Utc::now().timestamp();

    let payload = json!({
        "title": format!("{}", track.title),
        "artist": format!("{}", track.artist),
        "album": format!("{}", track.album),
        "arturl": format!("{}", track.arturl),
        "date": format!("{}", track.date),
        "duration": track.duration.as_secs_f64(),
        "timestamp": timestamp
    });

    let link = format!("{}/scrobble", CONFIG.tuunfm.link);
    let response = ureq::post(&link)
        .content_type("application/json")
        .send_json(payload);

    match response {
        Ok(r) => vpr!("Response: {r:#?}"),
        Err(e) => erm!("Response error: {e}"),
    }
}

pub fn scrobble(track: &Track) {
    if CONFIG.lastfm.used { lfm_scrobble(track) }
    if CONFIG.tuunfm.used { tfm_scrobble(track) }
}
