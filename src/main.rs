// main.rs

use std::io::{self, Write};

mod mpv;
mod track;
mod flags;
mod macros;

fn main() {
    print!("\x1b[?25l"); // ANSI escape code to hide the cursor
    io::stdout().flush().expect("Failed to flush stdout");
    flags::set_flags(false);
    mpv::launch_mpv();

    loop {
        let track = mpv::form_track();
        track.display();
        std::thread::sleep(std::time::Duration::from_millis(36));
    }
}
