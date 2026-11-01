mod board;
mod controller;
mod device;
mod display;
mod playback;
mod protocol;
mod soundboard;
mod tray;
mod voicemod;
mod voicemod_log;
mod window;

use std::sync::Mutex;
use tauri::{AppHandle, RunEvent};

/// Passed by the login item (autostart, still to come): the app then starts in the tray without opening its window
const AUTOSTART_ARG: &str = "--autostart";

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| window::open(app)))
        .manage(window::WindowState::default())
        .manage(Mutex::new(device::Status::default()))
        .manage(device::DisplayFrame::default())
        .invoke_handler(tauri::generate_handler![
            device::status,
            device::display_frame,
            window::window_ready
        ])
        .on_window_event(window::on_event)
        .setup(|app| {
            // A menu bar app: no Dock icon
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            tray::create(app.handle())?;
            device::start(app.handle().clone());

            if !std::env::args().any(|arg| arg == AUTOSTART_ARG) {
                window::open(app.handle());
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building the app")
        .run(on_run_event);
}

#[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
fn on_run_event(app: &AppHandle, event: RunEvent) {
    match event {
        // The app lives on with no window; Quit in the tray menu exits with a code
        RunEvent::ExitRequested { api, code: None, .. } => api.prevent_exit(),
        // Opened again from Finder while running
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { .. } => window::open(app),
        _ => {}
    }
}
