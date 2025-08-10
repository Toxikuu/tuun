use std::{
    sync::Arc,
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
use permitit::Permit;
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
};

#[instrument]
pub async fn authenticate_lastfm_scrobbler() -> Result<()> {
    let mut scrobbler_lock = SCROBBLER.lock().await;

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

        *scrobbler_lock = Some(Arc::new(scrobbler));
        info!("Authenticated lastfm scrobbler");
    } else {
        debug!("Not authenticating as scrobbler_lock is Some")
    }

    Ok(())
}

#[instrument(skip(track))]
pub async fn lastfm_now_playing(track: Track) -> Result<()> {
    let scrobbler_lock = SCROBBLER.lock().await;

    let Some(scrobbler) = &*scrobbler_lock else {
        error!("Scrobbler is not initialized");
        bail!("Scrobbler is not initialized");
    };

    let track = Scrobble::new(&track.artist, &track.title, &track.album);
    let scrobbler = Arc::clone(scrobbler);

    tokio::task::spawn_blocking(move || scrobbler.now_playing(&track)).await??;

    debug!("Set now playing");
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
    let scrobbler = Arc::clone(scrobbler);

    tokio::task::spawn_blocking(move || scrobbler.scrobble(&track)).await??;

    debug!("Scrobbled");
    Ok(())
}

#[instrument(skip(track))]
pub async fn discord_rpc(track: Track) -> Result<()> {
    if !CONFIG.discord.used {
        debug!("Skipping discord RPC as Discord is unused in the config");
        return Ok(());
    }

    // TODO: Maybe see if there's a better way of doing this check. For now this is addressed by
    // just lowering the socket connection timeout enough to where the block (which genuinely how
    // is it blocking while on another thread wtf) is not really noticeable.
    //
    // let socketpath = Path::new("/run/user/1000/discord-ipc-0");
    // if !socketpath.exists() {
    //     warn!("Discord IPC {socketpath:?} does not exist");
    //     return Ok(());
    // } // don't complain when discord is closed

    if track.is_default() {
        debug!("Cowardly refusing to set rich presence to a default track");
        return Ok(());
    } // don't try to set empty tracks

    tokio::spawn(async move {
        let mut client = RPC_CLIENT.lock().await;

        if let Err(e) = client.clear_activity() {
            error!("Failed to clear rich presence activity: {e}");

            warn!("Reconnecting Discord RPC client...");
            connect_discord_rpc_client().await;
        }

        let payload = create_rpc_payload(&track);
        debug!("Setting Discord RPC for {track:#?}");

        if let Err(e) = client.set_activity(payload) {
            drop(client);
            error!("Failed to set activity: {e}");

            connect_discord_rpc_client().await;
            client = RPC_CLIENT.lock().await;

            let payload = create_rpc_payload(&track);
            if let Err(e) = client.set_activity(payload) {
                error!("Failed to set activity after reconnect: {e:#}");
            }
        }

        Ok(())
    })
    .await?
}

#[instrument]
fn create_rpc_payload(track: &Track) -> Activity<'_> {
    debug!(track.arturl);
    let assets = activity::Assets::new()
        .large_image(&track.arturl)
        .large_text(&track.album)
        .small_image(CONFIG.discord.small_image.as_str())
        .small_text(CONFIG.discord.small_text.as_str());
    debug!("Created rich presence activity assets");

    let now = SystemTime::now();
    let duration = now
        .duration_since(UNIX_EPOCH)
        .expect("Grandfather paradox or something idk");

    let timestamp = activity::Timestamps::new().start(duration.as_secs() as i64);
    let payload = activity::Activity::new()
        .state(&track.artist)
        .details(&track.title)
        .assets(assets)
        .activity_type(activity::ActivityType::Listening)
        .timestamps(timestamp);
    debug!("Created rich presence activity payload");

    payload
}

#[instrument]
pub async fn connect_discord_rpc_client() {
    debug!("Attempting to connect Discord RPC client");

    let time = Duration::from_millis(CONFIG.discord.timeout);
    let lock = timeout(time, RPC_CLIENT.lock()).await;

    let Ok(mut client) = lock else {
        error!("Timed out while trying to acquire lock");
        return;
    };

    // attempt reconnection is recv fails
    if client.recv().is_err() {
        debug!("Failed to receive. Attempting to reconnect client.");
        if let Err(e) = client.close().permit(|e| matches!(e, DrpErr::NotConnected)) {
            error!("Failed to close IPC client: {e}");
            return;
        };
        if let Err(e) = client.connect() {
            error!("Failed to reconnect IPC client: {e}");
            return;
        };
    }

    if let Err(e) = client.connect() {
        error!("Failed to connect to Discord RPC client: {e}");
        return;
    }

    debug!("Connected Discord RPC client");
}
