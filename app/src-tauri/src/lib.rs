mod board;
mod protocol;
mod tray;

use board::{Board, BoardEvent};
use protocol::BoardMessage;
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WebviewUrl, WebviewWindowBuilder};

const WINDOW: &str = "main";

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
        .plugin(tauri_plugin_single_instance::init(|app, _, _| open_window(app)))
        .invoke_handler(tauri::generate_handler![board_status])
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
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building the app")
        .run(|_, event| {
            // Closing the window leaves the app running; Quit in the tray menu exits with a code
            if let RunEvent::ExitRequested { api, code: None, .. } = event {
                api.prevent_exit();
            }
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

/// Shows the window, creating it if needed: it only exists while open, to keep the app light.
fn open_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(WINDOW) {
        let _ = window.unminimize();
        let _ = window.set_focus();
        return;
    }
    // Building a window from a menu or single-instance callback deadlocks on Windows: use another thread
    let app = app.clone();
    std::thread::spawn(move || {
        let window = WebviewWindowBuilder::new(&app, WINDOW, WebviewUrl::default())
            .title("Drezimo Soundboard")
            .inner_size(480.0, 560.0)
            .min_inner_size(400.0, 440.0)
            .center()
            .build();
        if let Err(error) = window {
            eprintln!("cannot open the window: {error}");
        }
    });
}
