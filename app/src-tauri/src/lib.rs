mod board;
mod protocol;
mod tray;
mod window;

use board::{Board, BoardEvent};
use protocol::BoardMessage;
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State};

/// Passed when the app starts at login: it then stays in the tray instead of opening its window.
const AUTOSTART_ARG: &str = "--autostart";

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct BoardStatus {
    connected: bool,
    port: Option<String>,
    firmware: Option<String>,
}

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
            let (board, outgoing) = Board::new();
            app.manage(board);
            let handle = app.handle().clone();
            board::start(outgoing, move |event| on_board_event(&handle, event));

            if !std::env::args().any(|arg| arg == AUTOSTART_ARG) {
                window::open(app.handle());
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building the app")
        .run(|app, event| match event {
            // The app lives on with no window; Quit in the tray menu exits with a code
            RunEvent::ExitRequested { api, code: None, .. } => api.prevent_exit(),
            // Opened again from Finder while running
            #[cfg(target_os = "macos")]
            RunEvent::Reopen { .. } => window::open(app),
            _ => {}
        });
}

fn on_board_event(app: &AppHandle, event: BoardEvent) {
    match event {
        BoardEvent::Connected { port, firmware } => {
            // Development aid: the module's own LED copies pad 1, the one lit by the key echo below
            #[cfg(debug_assertions)]
            app.state::<Board>().send("DEVLED,MIRROR,0");
            set_status(app, BoardStatus { connected: true, port: Some(port), firmware: Some(firmware) });
        }
        BoardEvent::Disconnected => set_status(app, BoardStatus::default()),
        BoardEvent::Message(message) => {
            // Until the sound engine exists, a pressed pad simply lights up while held
            if let BoardMessage::Pad { index, pressed } = message {
                let level = if pressed { 255 } else { 0 };
                app.state::<Board>().send(format!("LED,{index},{level},{level},{level}"));
            }
            let _ = app.emit("board-message", message);
        }
    }
}

fn set_status(app: &AppHandle, status: BoardStatus) {
    tray::show_status(app, &status);
    let _ = app.emit("board-status", &status);
    *app.state::<Mutex<BoardStatus>>().lock().unwrap() = status;
}
