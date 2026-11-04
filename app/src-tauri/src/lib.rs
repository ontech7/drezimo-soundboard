mod board;
mod board_test;
mod controller;
mod custom_pages;
mod device;
mod display;
mod errors;
mod firmware;
mod last_page;
mod macros;
mod news;
mod overlay;
mod pictures;
mod playback;
mod press_again;
mod protocol;
mod report;
mod settings;
mod soundboard;
mod tour;
mod tray;
mod updates;
mod voicemod;
mod voicemod_bridge;
mod voicemod_log;
mod window;

use std::sync::Mutex;
use tauri::{AppHandle, Manager, RunEvent};

/// A string as Windows' wide-character functions take it, with its closing zero.
#[cfg(windows)]
pub(crate) fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

/// Passed by the login item the installer adds (windows/hooks.nsh): the app then starts in the tray, with no window
pub(crate) const AUTOSTART_ARG: &str = "--autostart";

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| window::open(app)))
        .manage(window::WindowState::default())
        .manage(Mutex::new(device::Status::default()))
        .manage(device::DisplayFrame::default())
        .manage(device::BoardViewState::default())
        .manage(device::BoardTestState::default())
        .manage(device::LastUse::default())
        .manage(custom_pages::LibraryState::default())
        .manage(updates::UpdateStateHolder::default())
        .invoke_handler(tauri::generate_handler![
            device::status,
            device::display_frame,
            device::board_view,
            device::press_pad,
            device::press_key,
            device::go_to_page,
            device::board_test,
            device::start_board_test,
            device::set_board_test_leds,
            device::set_board_test_display,
            device::restart_board_test,
            device::stop_board_test,
            device::set_display_settings,
            device::preview_led_brightness,
            device::preview_overlay,
            report::report,
            settings::settings,
            settings::set_settings,
            settings::about,
            settings::start_at_login,
            settings::set_start_at_login,
            custom_pages::custom_pages,
            custom_pages::set_custom_pages,
            custom_pages::library,
            press_again::press_outcomes,
            macros::macros,
            macros::set_macros,
            macros::try_macro,
            macros::stop_macro,
            news::news,
            news::dismiss_news,
            overlay::screens,
            updates::update_state,
            tour::tour_pictures,
            window::window_ready
        ])
        .register_asynchronous_uri_scheme_protocol(pictures::SCHEME, |context, request, responder| {
            let app = context.app_handle().clone();
            let path = request.uri().path().to_string();
            tauri::async_runtime::spawn(async move { responder.respond(pictures::respond(&app, &path).await) });
        })
        .on_window_event(window::on_event)
        .setup(|app| {
            // A menu bar app: no Dock icon
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            app.manage(settings::load(app.handle()));
            app.manage(pictures::Pictures::new(app.handle()));
            app.manage(custom_pages::load(app.handle()));
            app.manage(macros::load(app.handle()));
            app.manage(news::load(app.handle()));
            tray::create(app.handle())?;
            device::start(app.handle().clone());
            // A build made here does not replace itself with a published one
            if updates::OFFICIAL {
                updates::start(app.handle());
            }

            let tour_seen = app.state::<settings::SettingsState>().get().tour_seen;
            if !std::env::args().any(|arg| arg == AUTOSTART_ARG) || !tour_seen {
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
