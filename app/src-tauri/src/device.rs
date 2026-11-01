//! Brings the board and Voicemod together: the controller runs on its own thread, gets their events and sends them
//! its messages, and the app's status goes to the tray and the window.

use crate::board::{Board, BoardEvent, BoardStatus};
use crate::controller::{Controller, Output, PageSummary};
use crate::display::texts::Texts;
use crate::protocol;
use crate::tray;
use crate::voicemod::{Voicemod, VoicemodEvent, VoicemodStatus};
use crate::voicemod_log::{self, LogInput};
use serde::Serialize;
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager, State};

enum Input {
    Board(BoardEvent),
    Voicemod(VoicemodEvent),
    Log(LogInput),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub board: BoardStatus,
    pub voicemod: VoicemodStatus,
    /// Whether the playback shown comes from Voicemod's log, rather than from guesses
    pub voicemod_log: bool,
    pub page: Option<PageSummary>,
}

#[tauri::command]
pub fn status(status: State<Mutex<Status>>) -> Status {
    status.lock().unwrap().clone()
}

/// The display's current picture, in the hex of the board's DISPLAY message, for the window's preview
#[derive(Default)]
pub struct DisplayFrame(Mutex<String>);

#[tauri::command]
pub fn display_frame(frame: State<DisplayFrame>) -> String {
    frame.0.lock().unwrap().clone()
}

/// Starts looking for the board and Voicemod. Expects `Mutex<Status>` and `DisplayFrame` to be managed.
pub fn start(app: AppHandle) {
    let (inputs, received) = mpsc::channel();
    let (board, link) = Board::new();
    let board_inputs = inputs.clone();
    link.start(move |event| {
        let _ = board_inputs.send(Input::Board(event));
    });
    let log_inputs = inputs.clone();
    voicemod_log::follow(move |input| {
        let _ = log_inputs.send(Input::Log(input));
    });
    let voicemod = Voicemod::start(move |event| {
        let _ = inputs.send(Input::Voicemod(event));
    });
    thread::spawn(move || run(&app, &received, &board, &voicemod));
}

fn run(app: &AppHandle, inputs: &Receiver<Input>, board: &Board, voicemod: &Voicemod) {
    let mut controller = Controller::new(Texts::for_system());
    let mut status = Status::default();
    let mut published = status.clone();
    loop {
        let input = match controller.next_update() {
            Some(at) => match inputs.recv_timeout(at.saturating_duration_since(Instant::now())) {
                Ok(input) => Some(input),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => return,
            },
            None => match inputs.recv() {
                Ok(input) => Some(input),
                Err(_) => return,
            },
        };

        let now = Instant::now();
        match input {
            Some(Input::Board(BoardEvent::Status(board_status))) => {
                let connected = matches!(board_status, BoardStatus::Connected { .. });
                controller.set_board_connected(connected, now);
                // Development aid: the module's own LED copies pad 1
                #[cfg(debug_assertions)]
                if connected {
                    board.send("DEVLED,MIRROR,0");
                }
                status.board = board_status;
            }
            Some(Input::Board(BoardEvent::Message(message))) => {
                let _ = app.emit("board-message", &message);
                controller.board_message(message, now);
            }
            Some(Input::Voicemod(event)) => {
                if let VoicemodEvent::Status(voicemod_status) = event {
                    status.voicemod = voicemod_status;
                }
                controller.voicemod_event(event, now);
            }
            Some(Input::Log(input)) => controller.log_input(input, now),
            None => {}
        }

        controller.update(now);
        for output in controller.take_outputs() {
            match output {
                Output::Board(message) => board.send(message),
                Output::Voicemod(request) => voicemod.send(request),
                Output::Display(frame) => {
                    let hex = frame.hex();
                    if matches!(status.board, BoardStatus::Connected { .. }) {
                        board.send(protocol::display_message(&hex));
                    }
                    let _ = app.emit("display", &hex);
                    *app.state::<DisplayFrame>().0.lock().unwrap() = hex;
                }
            }
        }
        status.page = controller.page();
        status.voicemod_log = controller.follows_log();
        if status != published {
            publish(app, &status);
            published = status.clone();
        }
    }
}

fn publish(app: &AppHandle, status: &Status) {
    // Stored before it is announced, so a window asking for it in between gets the new one
    *app.state::<Mutex<Status>>().lock().unwrap() = status.clone();
    tray::show_status(app, status);
    let _ = app.emit("status", status);
}
