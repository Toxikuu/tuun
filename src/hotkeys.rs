use std::{
    collections::HashSet,
    sync::atomic::Ordering,
};

use anyhow::Result;
use tracing::{
    debug,
    error,
    info,
    trace,
    warn,
};
use winit::{
    application::ApplicationHandler,
    event::{
        DeviceEvent,
        ElementState,
        RawKeyEvent,
        WindowEvent,
    },
    event_loop::{
        ActiveEventLoop,
        ControlFlow,
        EventLoop,
    },
    keyboard::{
        KeyCode,
        PhysicalKey,
    },
};

use crate::mpv;

#[derive(Default)]
struct App {
    pressed_keys: HashSet<PhysicalKey>,
}

impl App {
    fn key_combo(&self, keys: &[PhysicalKey]) -> bool {
        self.pressed_keys == keys.iter().cloned().collect::<HashSet<_>>()
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) { debug!("App resumed") }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if event == WindowEvent::CloseRequested {
            warn!("Window was closed. Stopping...");
            event_loop.exit();
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::Key(RawKeyEvent { physical_key, state }) = event {
            match state {
                | ElementState::Pressed => {
                    trace!("Key {physical_key:?} pressed!");
                    self.pressed_keys.insert(physical_key);
                },
                | ElementState::Released => {
                    trace!("Key {physical_key:?} released!");
                    let _ = self.pressed_keys.remove(&physical_key);
                },
            }

            if let Err(e) = handle_hotkeys(self) {
                error!("Failed to execute hotkey action: {e}");
                debug!("Pressed keys: {:#?}", self.pressed_keys);
            }
        }
    }
}

macro_rules! pkey {
    ($key:ident) => {
        PhysicalKey::Code(KeyCode::$key)
    };
}

macro_rules! pkeys {
    ( $( $key:ident ),* ) => {
        &[
            $( pkey!($key), )*
        ]
    };
}

fn handle_hotkeys(app: &App) -> Result<()> {
    if app.key_combo(pkeys!(SuperLeft, AltLeft, KeyL)) {
        debug!("Loop registered by hotkey handler");
        if mpv::LOOPED.load(Ordering::Relaxed) {
            mpv::send_command_blocking(r#"{ "command": ["set", "loop-file", "no"] }"#)?;
        } else {
            mpv::send_command_blocking(r#"{ "command": ["set", "loop-file", "inf"] }"#)?;
        }
    } else if app.key_combo(pkeys!(SuperLeft, AltLeft, KeyM)) {
        debug!("Mute registered by hotkey handler");
        mpv::send_command_blocking(r#"{ "command": ["cycle", "mute"] }"#)?;
    } else if app.key_combo(pkeys!(SuperLeft, KeyK)) {
        debug!("Pause registered by hotkey handler");
        mpv::send_command_blocking(r#"{ "command": ["cycle", "pause"] }"#)?;
    } else if app.key_combo(pkeys!(SuperLeft, KeyL)) {
        debug!("Next registered by hotkey handler");
        mpv::send_command_blocking(r#"{ "command": ["playlist-next"] }"#)?;
    } else if app.key_combo(pkeys!(SuperLeft, KeyJ)) {
        debug!("Previous registered by hotkey handler");
        mpv::send_command_blocking(r#"{ "command": ["playlist-prev"] }"#)?;
    } else if app.key_combo(pkeys!(SuperLeft, Comma)) {
        debug!("Seek back registered by hotkey handler");
        mpv::send_command_blocking(r#"{ "command": ["seek", "-5", "relative", "exact"] }"#)?;
    } else if app.key_combo(pkeys!(SuperLeft, Period)) {
        debug!("Seek forward registered by hotkey handler");
        mpv::send_command_blocking(r#"{ "command": ["seek", "5", "relative", "exact"] }"#)?;
    }

    Ok(())
}

pub async fn register_global_hotkey_handler() {
    let Ok(event_loop) = EventLoop::new().map_err(|e| {
        warn!("Failed to create event loop: {e}");
        warn!("Global hotkeys will not work");
    }) else {
        return
    };

    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::default();
    info!("Registered global hotkey handler");

    if let Err(e) = event_loop.run_app(&mut app) {
        warn!("Failed to run event loop: {e}");
        warn!("Global hotkeys will not work");
    }
}
