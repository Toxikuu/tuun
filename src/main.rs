// main.rs
use delay::execute_after;
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

mod config;
mod delay;
mod flags;
mod macros;
mod mpv;
mod rpc;
mod scrobble;
mod track;

fn main() {
    let cfg = config::load_config();
    print!("\x1b[?25l"); // hide cursor
    io::stdout().flush().expect("Failed to flush stdout");
    flags::set_flags(false);
    mpv::launch_mpv();
    let client = Arc::new(Mutex::new(
        DiscordIpcClient::new(&cfg.discord.client_id).unwrap()
    ));
    client.lock().unwrap().connect().unwrap();

    let lfm = scrobble::LastFM::new(
        cfg.lastfm.apikey.to_string(),
        cfg.lastfm.secret.to_string(),
        cfg.lastfm.user.to_string(),
        cfg.lastfm.password.to_string(),
    );

    let mut current_track = String::new();
    let mut scrobble_task: Option<Arc<Mutex<bool>>> = None;
    let mut elapsed_time = 0.;

    loop {
        let track = mpv::form_track();
        track.display();
        std::thread::sleep(std::time::Duration::from_millis(cfg.general.polling_rate));

        if track.is_paused() == Some(true) { continue }
        let alt_track = track.title.clone();

        if current_track != alt_track {
            elapsed_time = 0.;

            if let Some(cancelled) = &scrobble_task {
                *cancelled.lock().unwrap() = true;
                erm!("Cancelled scrobble task for '{}'", current_track);
            }

            let lfm_copy = lfm.clone();
            let track_copy = track.clone();
            let track_copy2 = track.clone();

            rpc::set(track_copy2, &client).unwrap();

            let scrobble_delay = std::time::Duration::from_secs_f64(track.duration * 0.25);
            vpr!("Scrobbling in {:#?} seconds", scrobble_delay);
            
            scrobble_task = Some(execute_after(
                scrobble_delay,
                move || scrobble::scrobble(track_copy, lfm_copy)
            ));

            current_track = alt_track;
            continue
        }

        if track.is_looped() == Some(true) {
            elapsed_time += cfg.general.polling_rate as f64 / 1000.;

            if elapsed_time >= track.duration {
                elapsed_time = 0.;

                if let Some(cancelled) = &scrobble_task {
                    *cancelled.lock().unwrap() = true;
                    erm!("Cancelled previous scrobble task for looped '{}'", current_track);
                }

                let lfm_copy = lfm.clone();
                let track_copy = track.clone();

                let scrobble_delay = std::time::Duration::from_secs_f64(track.duration * 0.96);
                vpr!("Scrobbling looped track in {:#?} seconds", scrobble_delay);

                scrobble_task = Some(execute_after(
                    scrobble_delay,
                    move || scrobble::scrobble(track_copy, lfm_copy),
                ));
            }
            continue;
        }

        if !track.is_looped().unwrap_or(false) && scrobble_task.is_some() {
            if let Some(cancelled) = &scrobble_task {
                *cancelled.lock().unwrap() = true;
                erm!("Cancelled scrobble task for '{}'", current_track);
            }
            scrobble_task = None;
        }
    }
}
