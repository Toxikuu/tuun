use std::{
    collections::HashSet,
    sync::{
        LazyLock,
        Mutex,
        atomic::Ordering,
    },
    thread::sleep,
    time::Duration,
};

use anyhow::Result;
use rdev::{
    Event,
    EventType,
    Key,
    listen,
};
use tracing::{
    debug,
    error,
    info,
    trace,
    warn,
};

use crate::mpv;

static PRESSED_KEYS: LazyLock<Mutex<HashSet<Key>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

fn key_combo(keys: &[Key]) -> bool {
    let Ok(pressed) = PRESSED_KEYS.lock() else {
        warn!("Failed to lock PRESSED_KEYS");
        return false
    };
    *pressed == keys.iter().cloned().collect::<HashSet<_>>()
}

macro_rules! key {
    ($key:ident) => {
        Key::$key
    };
}

macro_rules! keys {
    ( $( $key:ident ),* ) => {
        &[
            $( key!($key), )*
        ]
    };
}

fn handle_hotkeys() -> Result<()> {
    if key_combo(keys!(MetaLeft, Alt, KeyL)) {
        debug!("Loop registered by hotkey handler");
        if mpv::LOOPED.load(Ordering::Relaxed) {
            mpv::send_command_blocking(r#"{ "command": ["set", "loop-file", "no"] }"#)?;
        } else {
            mpv::send_command_blocking(r#"{ "command": ["set", "loop-file", "inf"] }"#)?;
        }
    } else if key_combo(keys!(MetaLeft, Alt, KeyM)) {
        debug!("Mute registered by hotkey handler");
        mpv::send_command_blocking(r#"{ "command": ["cycle", "mute"] }"#)?;
    } else if key_combo(keys!(MetaLeft, KeyK)) {
        debug!("Pause registered by hotkey handler");
        mpv::send_command_blocking(r#"{ "command": ["cycle", "pause"] }"#)?;
    } else if key_combo(keys!(MetaLeft, KeyL)) {
        debug!("Next registered by hotkey handler");
        mpv::send_command_blocking(r#"{ "command": ["playlist-next"] }"#)?;
    } else if key_combo(keys!(MetaLeft, KeyJ)) {
        debug!("Previous registered by hotkey handler");
        mpv::send_command_blocking(r#"{ "command": ["playlist-prev"] }"#)?;
    } else if key_combo(keys!(MetaLeft, Comma)) {
        debug!("Seek back registered by hotkey handler");
        mpv::send_command_blocking(r#"{ "command": ["seek", "-5", "relative", "exact"] }"#)?;
    } else if key_combo(keys!(MetaLeft, Dot)) {
        debug!("Seek forward registered by hotkey handler");
        mpv::send_command_blocking(r#"{ "command": ["seek", "5", "relative", "exact"] }"#)?;
    }

    Ok(())
}

pub async fn register_global_hotkey_handler() {
    info!("Registered global hotkey handler");
    let callback = |event: Event| {
        match event.event_type {
            | EventType::KeyPress(key) => {
                trace!("Key {key:?} pressed");
                let Ok(mut keys) = PRESSED_KEYS.lock() else {
                    warn!("Failed to lock PRESSED_KEYS");
                    return
                };
                keys.insert(key);
            },
            | EventType::KeyRelease(key) => {
                trace!("Key {key:?} released");
                let Ok(mut keys) = PRESSED_KEYS.lock() else {
                    warn!("Failed to lock PRESSED_KEYS");
                    return
                };
                keys.remove(&key);
            },
            | _ => {
                trace!("Received untracked event")
            },
        }

        if let Err(e) = handle_hotkeys() {
            error!("Failed to execute hotkey action: {e}");
            let Ok(keys) = PRESSED_KEYS.lock() else {
                warn!("Failed to lock PRESSED_KEYS");
                return
            };
            debug!("Pressed keys: {keys:#?}")
        }
    };

    if let Err(e) = listen(callback) {
        warn!("Failed to start global hotkey listener: {e:?}");
        warn!("Global hotkeys will not work");
        loop {
            sleep(Duration::from_secs(5));
        }
    }
}
