// main.rs
use globals::CONFIG;
use delay::execute_after;
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

mod config;
mod delay;
mod flags;
mod globals;
mod macros;
mod mpv;
mod rpc;
mod scrobble;
mod track;

fn main() {
    print!("\x1b[?25l"); // hide cursor
    io::stdout().flush().expect("Failed to flush stdout");
    flags::set_flags(CONFIG.general.verbose);
    mpv::launch_mpv();
    let client = Arc::new(Mutex::new(
        DiscordIpcClient::new(&CONFIG.discord.client_id).unwrap()
    ));
    client.lock().unwrap().connect().unwrap();

    let mut current_track = String::new();
    let mut scrobble_task: Option<Arc<Mutex<bool>>> = None;
    let mut elapsed_time = 0.;

    loop {
        let track = mpv::form_track();
        track.display();
        std::thread::sleep(std::time::Duration::from_millis(CONFIG.general.polling_rate));
        
        if track.is_paused() == Some(true) { continue }
        let track_title = track.title.clone();

        if current_track != track_title {
            elapsed_time = 0.;

            if let Some(cancelled) = &scrobble_task {
                *cancelled.lock().unwrap() = true;
                erm!("Cancelled scrobble task for '{}'", current_track);
            }

            let track_copy = track.clone();
            let track_copy2 = track.clone();

            rpc::set(track_copy2, &client).unwrap();

            let scrobble_delay = std::time::Duration::from_secs_f64(track.duration * 0.25);
            vpr!("Scrobbling in {:#?} seconds", scrobble_delay);
            
            scrobble_task = Some(execute_after(
                scrobble_delay,
                move || scrobble::scrobble(&track_copy)
            ));

            current_track = track_title;
            continue
        }

        if track.is_looped() == Some(true) {
            vpr!("Detected track as looped");

            elapsed_time += CONFIG.general.polling_rate as f64 / 1000.;

            if elapsed_time >= track.duration {
                elapsed_time = 0.;

                if let Some(cancelled) = &scrobble_task {
                    *cancelled.lock().unwrap() = true;
                    erm!("Cancelled previous scrobble task for looped '{}'", current_track);
                }

                let track_copy = track.clone();

                let scrobble_delay = std::time::Duration::from_secs_f64(track.duration * 0.25);
                vpr!("Scrobbling looped track in {:#?} seconds", scrobble_delay);

                scrobble_task = Some(execute_after(
                    scrobble_delay,
                    move || scrobble::scrobble(&track_copy),
                ));
            }
            continue;
        }
    }
}
