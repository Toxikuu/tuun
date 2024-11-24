// main.rs
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use delay::execute_after;
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};

mod config;
mod scrobble;
mod mpv;
mod rpc;
mod delay;
mod track;
mod flags;
mod macros;

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
    // let mut rpc_task: Option<Arc<Mutex<bool>>> = None;

    loop {
        let track = mpv::form_track();
        track.display();
        std::thread::sleep(std::time::Duration::from_millis(cfg.general.polling_rate));

        if current_track != track.title {
            if let Some(cancelled) = &scrobble_task {
                *cancelled.lock().unwrap() = true;
                erm!("Cancelled scrobble task for '{}'", current_track);
            }

            let lfm_copy = lfm.clone();
            let track_copy = track.clone();
            let track_copy2 = track.clone();

            rpc::set(track_copy2, &client).unwrap();

            let scrobble_delay = std::time::Duration::from_secs_f64(track.duration / 4.);
            vpr!("Scrobbling in {:#?} seconds", scrobble_delay);
            
            scrobble_task = Some(execute_after(
                scrobble_delay,
                move || scrobble::scrobble(track_copy, lfm_copy)
            ));
        }

        current_track = track.title;
    }
}
