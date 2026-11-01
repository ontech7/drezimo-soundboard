//! The menu bar (macOS) or notification area (Windows) icon, where the app lives.

use crate::board::BoardStatus;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};

struct StatusItem(MenuItem<Wry>);

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let status = MenuItem::with_id(app, "status", status_text(&BoardStatus::default()), false, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Open Drezimo Soundboard", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&status, &PredefinedMenuItem::separator(app)?, &open, &quit])?;

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
    app.manage(StatusItem(status));
    Ok(())
}

pub fn show_status(app: &AppHandle, status: &BoardStatus) {
    let _ = app.state::<StatusItem>().0.set_text(status_text(status));
}

fn status_text(status: &BoardStatus) -> String {
    match status {
        BoardStatus::Searching => "Looking for the board…".to_string(),
        BoardStatus::Busy { .. } => "Board in use by another program".to_string(),
        BoardStatus::Connected { firmware, .. } => format!("Board connected · firmware {firmware}"),
    }
}
