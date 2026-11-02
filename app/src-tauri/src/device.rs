//! Brings the board and Voicemod together: the controller runs on its own thread, gets their events and sends them
//! its messages, and the app's status goes to the tray and the window.

use crate::board::{Board, BoardEvent, BoardStatus};
use crate::controller::{Controller, Output, PageSummary};
use crate::display::texts::Texts;
use crate::firmware::{self, FirmwareUpdate, Updater};
use crate::overlay::{self, Overlay};
use crate::protocol::{self, BoardMessage};
use crate::tray;
use crate::voicemod::{Voicemod, VoicemodEvent, VoicemodStatus};
use crate::voicemod_log::{self, LogInput};
use serde::Serialize;
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager, State};

enum Input {
    Board(BoardEvent),
    Voicemod(VoicemodEvent),
    Log(LogInput),
    /// The board's firmware was written, or why not
    Firmware(Result<(), String>),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub board: BoardStatus,
    pub voicemod: VoicemodStatus,
    /// Whether the playback shown comes from Voicemod's log, rather than from guesses
    pub voicemod_log: bool,
    pub page: Option<PageSummary>,
    pub firmware_update: Option<FirmwareUpdate>,
}

#[tauri::command]
pub fn status(status: State<Mutex<Status>>) -> Status {
    status.lock().unwrap().clone()
}

/// When a key or the knob was last used, if at all since the app started
#[derive(Default)]
pub struct LastUse(pub Mutex<Option<Instant>>);

/// The display's current picture, in the hex of the board's DISPLAY message, for the window's preview
#[derive(Default)]
pub struct DisplayFrame(Mutex<String>);

#[tauri::command]
pub fn display_frame(frame: State<DisplayFrame>) -> String {
    frame.0.lock().unwrap().clone()
}

/// Starts looking for the board and Voicemod. Expects `Mutex<Status>`, `DisplayFrame` and `LastUse` to be managed.
pub fn start(app: AppHandle) {
    let (inputs, received) = mpsc::channel();
    let (board, link) = Board::new();
    let board_inputs = inputs.clone();
    link.start(move |event| {
        let _ = board_inputs.send(Input::Board(event));
    });
    let log_inputs = inputs.clone();
    let firmware_inputs = inputs.clone();
    voicemod_log::follow(move |input| {
        let _ = log_inputs.send(Input::Log(input));
    });
    let voicemod = Voicemod::start(move |event| {
        let _ = inputs.send(Input::Voicemod(event));
    });
    let overlay = overlay::start(&app);
    thread::spawn(move || run(&app, &received, &firmware_inputs, &board, &voicemod, &overlay));
}

fn run(
    app: &AppHandle,
    inputs: &Receiver<Input>,
    firmware_inputs: &Sender<Input>,
    board: &Board,
    voicemod: &Voicemod,
    overlay: &Sender<Option<Overlay>>,
) {
    let mut controller = Controller::new(Texts::for_system());
    let mut status = Status::default();
    let mut published = status.clone();
    let mut updater = Updater::default();
    loop {
        let wake = controller.next_update().into_iter().chain(updater.next_update()).min();
        let input = match wake {
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
                if let BoardStatus::Connected { firmware, .. } = &board_status
                    && updater.board_connected(firmware)
                {
                    update_firmware(board, firmware, firmware_inputs);
                }
                // Development aid: the module's own LED copies pad 1
                #[cfg(debug_assertions)]
                if matches!(board_status, BoardStatus::Connected { .. }) {
                    board.send("DEVLED,MIRROR,0");
                }
                status.board = board_status;
            }
            Some(Input::Board(BoardEvent::Message(message))) => {
                let _ = app.emit("board-message", &message);
                let used = matches!(
                    message,
                    BoardMessage::Pad { .. }
                        | BoardMessage::Button { .. }
                        | BoardMessage::Encoder { .. }
                        | BoardMessage::EncoderButton { .. }
                );
                if used {
                    *app.state::<LastUse>().0.lock().unwrap() = Some(now);
                }
                controller.board_message(message, now);
            }
            Some(Input::Voicemod(event)) => {
                if let VoicemodEvent::Status(voicemod_status) = event {
                    status.voicemod = voicemod_status;
                }
                controller.voicemod_event(event, now);
            }
            Some(Input::Log(input)) => controller.log_input(input, now),
            Some(Input::Firmware(result)) => {
                if let Err(reason) = &result {
                    eprintln!("the board's firmware update failed: {reason}");
                }
                updater.installed(result, now);
            }
            None => {}
        }
        updater.update(now);
        // A board being updated restarts in its bootloader: nothing to drive meanwhile
        let updating = updater.state().as_ref().is_some_and(FirmwareUpdate::in_progress);
        let driven = matches!(status.board, BoardStatus::Connected { .. }) && !updating;
        controller.set_board_connected(driven, now);

        controller.update(now);
        for output in controller.take_outputs() {
            match output {
                Output::Board(message) => board.send(message),
                Output::Voicemod(request) => voicemod.send(request),
                Output::Display(frame) => {
                    let hex = frame.hex();
                    if driven {
                        board.send(protocol::display_message(&hex));
                    }
                    let _ = app.emit("display", &hex);
                    *app.state::<DisplayFrame>().0.lock().unwrap() = hex;
                }
                Output::Overlay(shown) => {
                    let _ = overlay.send(shown);
                }
            }
        }
        status.page = controller.page();
        status.voicemod_log = controller.follows_log();
        status.firmware_update = updater.state();
        if status != published {
            publish(app, &status);
            published = status.clone();
        }
    }
}

/// Sends the board to its bootloader and writes the app's firmware there, on a thread of its own: looking at the drives
/// can take a while.
fn update_firmware(board: &Board, current: &str, inputs: &Sender<Input>) {
    eprintln!("updating the board's firmware from {current} to {}", firmware::VERSION);
    let board = board.clone();
    let inputs = inputs.clone();
    thread::spawn(move || {
        // A bootloader drive there before BOOT belongs to another board
        let before = firmware::bootloader_drives();
        board.send("BOOT");
        let _ = inputs.send(Input::Firmware(firmware::install(&before)));
    });
}

fn publish(app: &AppHandle, status: &Status) {
    // Stored before it is announced, so a window asking for it in between gets the new one
    *app.state::<Mutex<Status>>().lock().unwrap() = status.clone();
    tray::show_status(app, status);
    let _ = app.emit("status", status);
}
