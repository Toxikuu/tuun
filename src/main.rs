// main.rs

#![deny(
    clippy::perf,
    clippy::todo,
    clippy::complexity,
)]
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    // clippy::unwrap_used,
    // clippy::expect_used,
    // clippy::panic,
    unused,
    // missing_docs,
    // clippy::cargo,
)]

use globals::CONFIG;
use delay::execute_after;
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use track::Track;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread::sleep;

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
    let mut elapsed_time = Duration::ZERO;

    loop {
        let track = mpv::form_track();
        track.display();
        sleep(CONFIG.general.polling_rate_dur);
        
        if Track::is_paused() == Some(true) { continue }
        let track_title = track.title.clone();

        if current_track != track_title {
            elapsed_time = Duration::ZERO;

            if let Some(cancelled) = &scrobble_task {
                *cancelled.lock().unwrap() = true;
                erm!("Cancelled scrobble task for '{}'", current_track);
            }

            rpc::set(&track, &client).unwrap_or_else(|_|
                erm!("Rip discord ipc socket 😔")
            );

            let scrobble_delay = track.duration.mul_f64(0.25);
            vpr!("Scrobbling in {:#?} seconds", scrobble_delay);
            
            scrobble_task = Some(execute_after(
                scrobble_delay,
                move || scrobble::scrobble(&track)
            ));

            current_track = track_title;
            continue
        }

        if Track::is_looped() == Some(true) {
            vpr!("Detected track as looped");

            elapsed_time += CONFIG.general.polling_rate_dur;

            if elapsed_time >= track.duration {
                elapsed_time = Duration::ZERO;

                if let Some(cancelled) = &scrobble_task {
                    *cancelled.lock().unwrap() = true;
                    erm!("Cancelled previous scrobble task for looped '{}'", current_track);
                }

                let track_copy = track.clone();

                let scrobble_delay = track.duration.mul_f64(0.25);
                vpr!("Scrobbling looped track in {:#?} seconds", scrobble_delay);

                scrobble_task = Some(execute_after(
                    scrobble_delay,
                    move || scrobble::scrobble(&track_copy),
                ));
            }
        }
    }
}
