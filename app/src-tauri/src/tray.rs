//! The menu bar (macOS) or notification area (Windows) icon, where the app lives.

use crate::board::BoardStatus;
use crate::device::Status;
use crate::firmware::FirmwareUpdate;
use crate::voicemod::VoicemodStatus;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};

struct StatusItems {
    board: MenuItem<Wry>,
    voicemod: MenuItem<Wry>,
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let status = Status::default();
    let board = MenuItem::with_id(app, "board", board_text(&status.board, None), false, None::<&str>)?;
    let voicemod = MenuItem::with_id(app, "voicemod", voicemod_text(status.voicemod), false, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Open Drezimo Soundboard", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&board, &voicemod, &separator, &open, &quit])?;

    // macOS draws a black template image in the menu bar colour; Windows shows the app icon
    #[cfg(target_os = "macos")]
    let icon = tauri::include_image!("icons/tray-template.png");
    #[cfg(not(target_os = "macos"))]
    let icon = app.default_window_icon().cloned().expect("the bundle has an icon");

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("Drezimo Soundboard")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => crate::window::open(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    app.manage(StatusItems { board, voicemod });
    Ok(())
}

pub fn show_status(app: &AppHandle, status: &Status) {
    let items = app.state::<StatusItems>();
    let _ = items
        .board
        .set_text(board_text(&status.board, status.firmware_update.as_ref()));
    let _ = items.voicemod.set_text(voicemod_text(status.voicemod));
}

fn board_text(status: &BoardStatus, update: Option<&FirmwareUpdate>) -> String {
    match update {
        Some(FirmwareUpdate::Updating { .. }) => return "Updating the board…".to_string(),
        Some(FirmwareUpdate::Restarting { .. }) => return "Restarting the board…".to_string(),
        _ => {}
    }
    match status {
        BoardStatus::Searching => "Looking for the board…".to_string(),
        BoardStatus::Busy { .. } => "Board in use by another program".to_string(),
        BoardStatus::Connected { firmware, .. } => format!("Board connected · firmware {firmware}"),
    }
}

fn voicemod_text(status: VoicemodStatus) -> &'static str {
    match status {
        VoicemodStatus::Connecting => "Connecting to Voicemod…",
        VoicemodStatus::Connected => "Voicemod connected",
        VoicemodStatus::Unreachable => "Voicemod is not running",
        VoicemodStatus::KeyRejected => "Voicemod refused the connection",
        VoicemodStatus::MissingKey => "Built without a Voicemod key",
    }
}
