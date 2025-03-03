use anyhow::Result;
use std::{collections::HashSet, sync::atomic::Ordering};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, ElementState, RawKeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
};

use crate::mpv;

#[derive(Default)]
struct App {
    pressed_keys: HashSet<PhysicalKey>,
}

impl App {
    fn is_key_held(&self, key: PhysicalKey) -> bool {
        self.pressed_keys.contains(&key)
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        println!("App resumed")
    }

    fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            _window_id: winit::window::WindowId,
            event: winit::event::WindowEvent,
        ) {
        if event == WindowEvent::CloseRequested {
            println!("Window was closed. Stopping...");
            event_loop.exit();
        }
    }

    fn device_event(
            &mut self,
            _event_loop: &ActiveEventLoop,
            _device_id: winit::event::DeviceId,
            event: DeviceEvent,
        ) {
        if let DeviceEvent::Key( RawKeyEvent { physical_key, state } ) = event {
            match state {
                ElementState::Pressed => {
                    println!("\x1b[36;1mKey {physical_key:?} pressed!\x1b[0m");
                    self.pressed_keys.insert(physical_key);
                }
                ElementState::Released => {
                    println!("\x1b[35;1mKey {physical_key:?} released!\x1b[0m");
                    let _ = self.pressed_keys.remove(&physical_key);
                }
            }

            if let Err(e) = handle_hotkeys(self) {
                eprintln!("Failed to execute hotkey action: {e}");
                eprintln!("Pressed keys: {:#?}", self.pressed_keys);
            }
        }
    }
}

fn handle_hotkeys(app: &App) -> Result<()> {
    if app.is_key_held(PhysicalKey::Code(KeyCode::SuperLeft)) {
        if app.is_key_held(PhysicalKey::Code(KeyCode::AltLeft)) {
            if app.is_key_held(PhysicalKey::Code(KeyCode::KeyL)) {
                println!("\x1b[34;1mLoop registered by hotkey handler\x1b[0m");
                if mpv::LOOPED.load(Ordering::Relaxed) {
                    mpv::send_command_blocking(r#"{ "command": ["set", "loop-file", "no"] }"#)?;
                } else {
                    mpv::send_command_blocking(r#"{ "command": ["set", "loop-file", "inf"] }"#)?;
                }
            }
            else if app.is_key_held(PhysicalKey::Code(KeyCode::KeyM)) {
                println!("\x1b[34;1mMute registered by hotkey handler\x1b[0m");
                mpv::send_command_blocking(r#"{ "command": ["cycle", "mute"] }"#)?;
            }
        }

        else if app.is_key_held(PhysicalKey::Code(KeyCode::KeyK)) {
            println!("\x1b[34;1mPause registered by hotkey handler\x1b[0m");
            mpv::send_command_blocking(r#"{ "command": ["cycle", "pause"] }"#)?;
        }
        else if app.is_key_held(PhysicalKey::Code(KeyCode::KeyL)) {
            println!("\x1b[34;1mNext registered by hotkey handler\x1b[0m");
            mpv::send_command_blocking(r#"{ "command": ["playlist-next"] }"#)?;
        }
        else if app.is_key_held(PhysicalKey::Code(KeyCode::KeyJ)) {
            println!("\x1b[34;1mPrevious registered by hotkey handler\x1b[0m");
            mpv::send_command_blocking(r#"{ "command": ["playlist-prev"] }"#)?;
        }
        else if app.is_key_held(PhysicalKey::Code(KeyCode::Comma)) {
            println!("\x1b[34;1mPrevious registered by hotkey handler\x1b[0m");
            mpv::send_command_blocking(r#"{ "command": ["seek", "-5", "relative", "exact"] }"#)?;
        }
        else if app.is_key_held(PhysicalKey::Code(KeyCode::Period)) {
            println!("\x1b[34;1mPrevious registered by hotkey handler\x1b[0m");
            mpv::send_command_blocking(r#"{ "command": ["seek", "5", "relative", "exact"] }"#)?;
        }
    }

    Ok(())
}

pub async fn register_global_hotkey_handler() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::default();
    println!("\x1b[31;1mRegistered global hotkey handler\x1b[0m");
    event_loop.run_app(&mut app).unwrap();
}
