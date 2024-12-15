// rpc.rs
//
// handles discord rpc

use crate::globals::CONFIG;
use crate::vpr;
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::track::Track;
use std::sync::{Arc, Mutex};

pub fn set(track: Track, client: &Arc<Mutex<DiscordIpcClient>>) -> Result<(), Box<dyn Error>> {
    if !CONFIG.discord.used { return Ok(()) } // maybe return Err("Unused")

    let mut client = client.lock().unwrap();
    client.clear_activity()?;
    vpr!("Cleared rpc activity");

    let assets = activity::Assets::new()
        .large_image(&track.arturl)
        .large_text(&track.album)
        .small_image("pfp")
        .small_text("hello :3");

    let now = SystemTime::now();
    let duration = now.duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    let timestamp = activity::Timestamps::new().start(duration.as_secs().try_into().unwrap());

    let payload = activity::Activity::new()
        .state(&track.artist)
        .details(&track.title)
        .assets(assets)
        .activity_type(activity::ActivityType::Listening)
        .timestamps(timestamp)
    ;

    vpr!("Attempting to set discord activity");
    client.set_activity(payload).expect("Failed to set activity");
    vpr!("Supposedly set activity. Sleeping...");
    
    Ok(())
}
