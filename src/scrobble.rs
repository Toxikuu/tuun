// scrobble.rs
//
// responsible for scrobbling

use rustfm_scrobble::{Scrobble, Scrobbler};
use crate::track::Track;
use crate::vpr;

#[derive(Debug, Clone)]
pub struct LastFM {
    pub apikey: String,
    pub secret: String,
    pub username: String,
    pub password: String,
}

impl LastFM {
    pub fn new(apikey: String, secret: String, username: String, password: String) -> Self {
        Self { apikey, secret, username, password }
    }
}

pub fn scrobble(track: Track, lfm: LastFM) {
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
