use std::{
    path::Path,
    time::{
        Duration,
        SystemTime,
        UNIX_EPOCH,
    },
};

use anyhow::{
    Result,
    bail,
};
use discord_rich_presence::{
    DiscordIpc,
    activity::{
        self,
        Activity,
    },
    error::Error as DrpErr,
};
use rustfm_scrobble::{
    Scrobble,
    Scrobbler,
};
use tokio::time::timeout;
use tracing::{
    debug,
    error,
    info,
    instrument,
    warn,
};

use crate::{
    CONFIG,
    RPC_CLIENT,
    SCROBBLER,
    structs::{
        LastFM,
        Track,
    },
    traits::Permit,
};

#[instrument]
pub async fn authenticate_lastfm_scrobbler() -> Result<()> {
    let mut scrobbler_lock = crate::SCROBBLER.lock().await;

    if scrobbler_lock.is_none() {
        info!("Authenticating lastfm scrobbler...");
        let lfm = LastFM::new();

        if lfm.apikey.is_empty()
            || lfm.secret.is_empty()
            || lfm.username.is_empty()
            || lfm.password.is_empty()
        {
            warn!("Cowardly refusing to authenticate without credentials");
            bail!("Cowardly refusing to authenticate without credentials");
        }

        let mut scrobbler = Scrobbler::new(lfm.apikey, lfm.secret);
        scrobbler.authenticate_with_password(lfm.username, lfm.password)?;

        *scrobbler_lock = Some(scrobbler);
        info!("Authenticated lastfm scrobbler");
    } else {
        debug!("Not authenticating as scrobbler_lock is Some")
    }

    Ok(())
}

#[instrument(skip(track))]
pub async fn lastfm_scrobble(track: Track) -> Result<()> {
    let scrobbler_lock = SCROBBLER.lock().await;

    let Some(scrobbler) = &*scrobbler_lock else {
        error!("Scrobbler is not initialized");
        bail!("Scrobbler is not initialized");
    };

    let track = Scrobble::new(&track.artist, &track.title, &track.album);

    scrobbler.now_playing(&track)?;
    debug!("Set lastfm now playing to {track:#?}");
    scrobbler.scrobble(&track)?;
    debug!("Scrobbled {track:#?}");

    Ok(())
}

#[instrument(skip(track))]
pub async fn discord_rpc(track: Track) -> Result<()> {
    if !CONFIG.discord.used {
        debug!("Skipping discord RPC as Discord is unused in the config");
        return Ok(())
    }

    let socketpath = Path::new("/run/user/1000/discord-ipc-0");
    if !socketpath.exists() {
        warn!("Discord IPC {socketpath:?} does not exist");
        return Ok(())
    } // don't complain when discord is closed
    if track.is_default()   {
        debug!("Cowardly refusing to set rich presence to a default track");
        return Ok(())
    } // don't try to set empty tracks
    
    // blocking is used here because it's fast
    // TODO: migrate this to async
    tokio::task::spawn_blocking(move || {
        let mut client = RPC_CLIENT.lock().unwrap();

        if let Err(e) = client.clear_activity() {
            error!("Failed to clear rich presence activity: {e:#}");
        }

        let assets = activity::Assets::new()
            .large_image(&track.arturl)
            .large_text(&track.album)
            .small_image("pfp")
            .small_text("hello from tuun!");

        debug!("Created rich presence activity assets");

        let now = SystemTime::now();
        let duration = now.duration_since(UNIX_EPOCH)?;

        let timestamp = activity::Timestamps::new().start(duration.as_secs().try_into().unwrap());
        let payload = activity::Activity::new()
            .state(&track.artist)
            .details(&track.title)
            .assets(assets)
            .activity_type(activity::ActivityType::Listening)
            .timestamps(timestamp);

        debug!("Setting Discord RPC for {track:#?}");

        if let Err(e) = client.set_activity(payload) {
            error!("Failed set activity: {e:#}");
        }
        Ok(())
    }).await?
}
