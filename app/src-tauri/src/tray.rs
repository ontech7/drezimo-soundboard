//! The menu bar (macOS) or notification area (Windows) icon, where the app lives.

use crate::board::BoardStatus;
use crate::device::Status;
use crate::firmware::FirmwareUpdate;
use crate::settings::{Language, SettingsState};
use crate::voicemod::VoicemodStatus;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

struct Items {
    board: MenuItem<Wry>,
    voicemod: MenuItem<Wry>,
    open: MenuItem<Wry>,
    quit: MenuItem<Wry>,
}

struct Words {
    open: &'static str,
    quit: &'static str,
    updating: &'static str,
    restarting: &'static str,
    searching: &'static str,
    busy: &'static str,
    /// Followed by the firmware's version
    connected: &'static str,
    voicemod_connecting: &'static str,
    voicemod_connected: &'static str,
    voicemod_unreachable: &'static str,
    voicemod_key_rejected: &'static str,
    voicemod_missing_key: &'static str,
}

const ENGLISH: Words = Words {
    open: "Open Drezimo Soundboard",
    quit: "Quit",
    updating: "Updating the board…",
    restarting: "Restarting the board…",
    searching: "Looking for the board…",
    busy: "Board in use by another program",
    connected: "Board connected · firmware",
    voicemod_connecting: "Connecting to Voicemod…",
    voicemod_connected: "Voicemod connected",
    voicemod_unreachable: "Voicemod is not running",
    voicemod_key_rejected: "Voicemod refused the connection",
    voicemod_missing_key: "Built without a Voicemod key",
};

const ITALIAN: Words = Words {
    open: "Apri Drezimo Soundboard",
    quit: "Esci",
    updating: "Aggiornamento della board…",
    restarting: "Riavvio della board…",
    searching: "Ricerca della board…",
    busy: "Board usata da un altro programma",
    connected: "Board collegata · firmware",
    voicemod_connecting: "Connessione a Voicemod…",
    voicemod_connected: "Voicemod collegato",
    voicemod_unreachable: "Voicemod non è aperto",
    voicemod_key_rejected: "Voicemod ha rifiutato la connessione",
    voicemod_missing_key: "Compilata senza chiave di Voicemod",
};

fn words(language: Language) -> &'static Words {
    match language.resolved() {
        Language::Italian => &ITALIAN,
        _ => &ENGLISH,
    }
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let words = words(app.state::<SettingsState>().get().language);
    let status = Status::default();
    let board = MenuItem::with_id(app, "board", board_text(words, &status), false, None::<&str>)?;
    let voicemod = MenuItem::with_id(app, "voicemod", voicemod_text(words, &status), false, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", words.open, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", words.quit, true, None::<&str>)?;
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
        // As on Windows: the menu on a right click, the window on a double click. A menu bar app shows its menu on any click
        .show_menu_on_left_click(cfg!(target_os = "macos"))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } = event
            {
                crate::window::open(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => crate::window::open(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    app.manage(Items {
        board,
        voicemod,
        open,
        quit,
    });
    Ok(())
}

pub fn show_status(app: &AppHandle, status: &Status) {
    let words = words(app.state::<SettingsState>().get().language);
    let items = app.state::<Items>();
    let _ = items.board.set_text(board_text(words, status));
    let _ = items.voicemod.set_text(voicemod_text(words, status));
}

/// The fixed items; the device thread redoes the status items.
pub fn set_language(app: &AppHandle, language: Language) {
    let words = words(language);
    let items = app.state::<Items>();
    let _ = items.open.set_text(words.open);
    let _ = items.quit.set_text(words.quit);
}

fn board_text(words: &Words, status: &Status) -> String {
    match status.firmware_update {
        Some(FirmwareUpdate::Updating { .. }) => return words.updating.to_string(),
        Some(FirmwareUpdate::Restarting { .. }) => return words.restarting.to_string(),
        _ => {}
    }
    match &status.board {
        BoardStatus::Searching => words.searching.to_string(),
        BoardStatus::Busy { .. } => words.busy.to_string(),
        BoardStatus::Connected { firmware, .. } => format!("{} {firmware}", words.connected),
    }
}

fn voicemod_text(words: &Words, status: &Status) -> &'static str {
    match status.voicemod {
        VoicemodStatus::Connecting => words.voicemod_connecting,
        VoicemodStatus::Connected => words.voicemod_connected,
        VoicemodStatus::Unreachable => words.voicemod_unreachable,
        VoicemodStatus::KeyRejected => words.voicemod_key_rejected,
        VoicemodStatus::MissingKey => words.voicemod_missing_key,
    }
}
