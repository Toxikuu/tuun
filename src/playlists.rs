// src/playlists.rs
//! Some logic for handling playlists

use std::{
    fs,
    path::PathBuf,
};

use tracing::{
    debug,
    instrument,
    trace,
    warn,
};

use crate::CONFIG;

#[derive(Debug)]
pub struct Playlist {
    playlist_path: PathBuf,
}

impl Playlist {
    #[instrument]
    pub fn new(playlist_path: PathBuf) -> Self {
        let pl = Self { playlist_path };
        debug!("Created new playlist: {pl:#?}");
        pl
    }

    #[instrument(level = "trace")]
    pub fn write(&self, songs: &[PathBuf]) {
        let contents = songs
            .iter()
            .map(|song| song.to_string_lossy())
            .collect::<Vec<_>>()
            .join("\n");

        fs::write(&self.playlist_path, &contents).expect("Failed to write playlist");
        debug!("Wrote playlist: {self:#?}");
        trace!("Playlist contents: {contents}");
    }
}

#[instrument]
pub fn create_all_playlist() {
    let path = PathBuf::from("/tmp/tuun/all.tpl");

    // only recreate all.tpl on restarts since it resides in /tmp
    if path.exists() {
        return;
    }

    debug!("Creating the all playlist...");
    let all_playlist = Playlist::new(path);

    let songs = fs::read_dir(&CONFIG.general.music_dir)
        .expect("Failed to read music directory")
        .map_while(Result::ok)
        .map(|e| e.path())
        .collect::<Vec<_>>();

    all_playlist.write(&songs);
}
