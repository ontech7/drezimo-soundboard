mod board;
mod protocol;
mod tray;
mod window;

use board::{Board, BoardEvent, BoardStatus};
use protocol::BoardMessage;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State};

/// Passed by the login item (autostart, still to come): the app then starts in the tray without opening its window
const AUTOSTART_ARG: &str = "--autostart";

#[tauri::command]
fn board_status(status: State<Mutex<BoardStatus>>) -> BoardStatus {
    status.lock().unwrap().clone()
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| window::open(app)))
        .manage(window::WindowState::default())
        .invoke_handler(tauri::generate_handler![board_status, window::window_ready])
        .on_window_event(window::on_event)
        .setup(|app| {
            // A menu bar app: no Dock icon
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            app.manage(Mutex::new(BoardStatus::default()));
            tray::create(app.handle())?;
            let (board, link) = Board::new();
            app.manage(board);
            let handle = app.handle().clone();
            link.start(move |event| on_board_event(&handle, event));

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

fn on_board_event(app: &AppHandle, event: BoardEvent) {
    match event {
        BoardEvent::Status(status) => {
            // Development aid: the module's own LED copies pad 1, the one lit by the key echo below
            #[cfg(debug_assertions)]
            if matches!(status, BoardStatus::Connected { .. }) {
                app.state::<Board>().send("DEVLED,MIRROR,0");
            }
            set_status(app, status);
        }
        BoardEvent::Message(message) => {
            // Until the sound engine exists, a pressed pad simply lights up while held
            if let BoardMessage::Pad { index, pressed } = message {
                let level = if pressed { 255 } else { 0 };
                app.state::<Board>()
                    .send(format!("LED,{index},{level},{level},{level}"));
            }
            let _ = app.emit("board-message", message);
        }
    }
}

fn set_status(app: &AppHandle, status: BoardStatus) {
    // Stored before it is announced, so a window asking for it in between gets the new status
    *app.state::<Mutex<BoardStatus>>().lock().unwrap() = status.clone();
    tray::show_status(app, &status);
    let _ = app.emit("board-status", &status);
}
