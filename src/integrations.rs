use anyhow::{bail, Result};
use discord_rich_presence::{activity, DiscordIpc};
use crate::{p, CONFIG, RPC_CLIENT, SCROBBLER};
use crate::structs::{Track, LastFM};
use rustfm_scrobble::{Scrobble, Scrobbler};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn authenticate_scrobbler() -> Result<()> {
    let mut scrobbler_lock = crate::SCROBBLER.lock().unwrap();

    if scrobbler_lock.is_none() {
        let lfm = LastFM::new();

        let mut scrobbler = Scrobbler::new(lfm.apikey, lfm.secret);
        scrobbler.authenticate_with_password(lfm.username, lfm.password)?;

        *scrobbler_lock = Some(scrobbler);
    }

    Ok(())
}

pub fn lastfm_scrobble(track: Track) -> Result<()> {
    let scrobbler_lock = SCROBBLER.lock().unwrap();
    
    let Some(scrobbler) = &*scrobbler_lock else {
        bail!("Scrobbler is not initialized")
    };

    let track = Scrobble::new(&track.artist, &track.title, &track.album);

    scrobbler.now_playing(&track)?;
    scrobbler.scrobble(&track)?;

    Ok(())
}

pub async fn discord_rpc(track: Track) -> Result<()> {
    if !CONFIG.discord.used { return Ok(()) }

    let socketpath = Path::new("/run/user/1000/discord-ipc-0");
    if !socketpath.exists() { return Ok(()) } // don't complain when discord is closed
    if track.is_default()   { return Ok(()) } // don't try to set empty tracks
    
    // blocking is used here because it's fast
    // TODO: migrate this to async
    tokio::task::spawn_blocking(move || {
        let mut client = RPC_CLIENT.lock().unwrap();

        if let Err(e) = client.clear_activity() {
            // log::error!("Failed to clear activity: {e:#}");
        }
        let assets = activity::Assets::new()
            .large_image(&track.arturl)
            .large_text(&track.album)
            .small_image("pfp")
            .small_text("hello from tuun!");

        let now = SystemTime::now();
        let duration = now.duration_since(UNIX_EPOCH)?;

        let timestamp = activity::Timestamps::new().start(duration.as_secs().try_into().unwrap());
        let payload = activity::Activity::new()
            .state(&track.artist)
            .details(&track.title)
            .assets(assets)
            .activity_type(activity::ActivityType::Listening)
            .timestamps(timestamp);

        p!("Setting Discord RPC for {track:#?}");

        if let Err(e) = client.set_activity(payload) {
            // log::error!("Failed set activity: {e:#}");
        }
        Ok(())
    }).await?
}
