//! Brings the board and Voicemod together: the controller runs on its own thread, gets their events and sends them
//! its messages, and the app's status goes to the tray and the window.

use crate::board::{Board, BoardEvent, BoardStatus};
use crate::controller::{Controller, Output, PageSummary};
use crate::firmware::{self, FirmwareUpdate, Updater};
use crate::overlay::{self, Look, OverlayUpdate};
use crate::protocol::{self, BoardMessage, DisplaySettings};
use crate::settings::{OverlaySettings, Settings, SettingsState};
use crate::tray;
use crate::voicemod::{Voicemod, VoicemodEvent, VoicemodStatus};
use crate::voicemod_log::{self, LogInput};
use serde::Serialize;
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};

enum Input {
    Board(BoardEvent),
    Voicemod(VoicemodEvent),
    Log(LogInput),
    /// The board's firmware was written, or why not
    Firmware(Result<(), String>),
    /// From the window
    Settings(Settings),
    DisplaySettings(DisplaySettings),
    LedBrightnessPreview(u8),
    OverlayPreview,
}

/// The way in for the window's commands
pub struct Inputs(Mutex<Sender<Input>>);

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub board: BoardStatus,
    pub voicemod: VoicemodStatus,
    /// Whether the playback shown comes from Voicemod's log, rather than from guesses
    pub voicemod_log: bool,
    pub page: Option<PageSummary>,
    pub firmware_update: Option<FirmwareUpdate>,
    /// As the connected board keeps them
    pub display_settings: Option<DisplaySettings>,
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

/// Starts looking for the board and Voicemod. Expects `Mutex<Status>`, `DisplayFrame`, `LastUse` and `SettingsState` to be
/// managed.
pub fn start(app: AppHandle) {
    let (inputs, received) = mpsc::channel();
    app.manage(Inputs(Mutex::new(inputs.clone())));
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
    overlay: &Sender<OverlayUpdate>,
) {
    let mut settings = app.state::<SettingsState>().get();
    let mut controller = Controller::new(settings.language.texts());
    controller.set_overlay_time(overlay_time(&settings.overlay));
    let _ = overlay.send(OverlayUpdate::Look(look(&settings.overlay)));
    let mut status = Status::default();
    let mut published = status.clone();
    let mut updater = Updater::default();
    // SETTINGS sent and not answered yet: only the answer to the last one is current
    let mut settings_reads = 0;
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
                let was_connected = matches!(status.board, BoardStatus::Connected { .. });
                if matches!(board_status, BoardStatus::Connected { .. }) && !was_connected {
                    // The board forgets the LEDs' brightness when unplugged; it keeps the display's settings
                    board.send(brightness_message(settings.led_brightness));
                    board.send("SETTINGS");
                    settings_reads = 1;
                    // Development aid: the module's own LED copies pad 1
                    #[cfg(debug_assertions)]
                    board.send("DEVLED,MIRROR,0");
                }
                if !matches!(board_status, BoardStatus::Connected { .. }) {
                    status.display_settings = None;
                }
                status.board = board_status;
            }
            Some(Input::Board(BoardEvent::Message(BoardMessage::Settings(saved)))) => {
                settings_reads = u32::saturating_sub(settings_reads, 1);
                if settings_reads == 0 {
                    status.display_settings = Some(saved);
                }
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
            Some(Input::Settings(new)) => {
                controller.set_texts(new.language.texts());
                controller.set_overlay_time(overlay_time(&new.overlay));
                let _ = overlay.send(OverlayUpdate::Look(look(&new.overlay)));
                if new.led_brightness != settings.led_brightness {
                    board.send(brightness_message(new.led_brightness));
                }
                settings = new;
                // In the new language, after any update already on its way in the old one
                tray::show_status(app, &status);
            }
            Some(Input::DisplaySettings(wanted)) => {
                if let Some(saved) = status.display_settings {
                    for message in saved.changes(&wanted) {
                        board.send(message);
                    }
                    // Read back: the board has the last word
                    board.send("SETTINGS");
                    settings_reads += 1;
                    status.display_settings = Some(wanted);
                }
            }
            Some(Input::LedBrightnessPreview(percent)) => board.send(brightness_message(percent)),
            Some(Input::OverlayPreview) => controller.preview_overlay(now),
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
                    let _ = overlay.send(OverlayUpdate::Content(shown));
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

fn brightness_message(percent: u8) -> String {
    format!("BRIGHT,{percent}")
}

fn overlay_time(overlay: &OverlaySettings) -> Option<Duration> {
    overlay.enabled.then(|| Duration::from_secs(overlay.seconds.into()))
}

fn look(overlay: &OverlaySettings) -> Look {
    Look {
        anchor: overlay.anchor,
        opacity: f32::from(overlay.opacity) / 100.0,
    }
}

fn send(app: &AppHandle, input: Input) {
    if let Some(inputs) = app.try_state::<Inputs>() {
        let _ = inputs.0.lock().unwrap().send(input);
    }
}

pub fn apply_settings(app: &AppHandle, settings: Settings) {
    send(app, Input::Settings(settings));
}

/// Within what the firmware takes: minutes up to a day
#[tauri::command]
pub fn set_display_settings(app: AppHandle, settings: DisplaySettings) {
    let settings = DisplaySettings {
        brightness: settings.brightness.min(100),
        dim_after: settings.dim_after.min(1440),
        off_after: settings.off_after.min(1440),
        shift_every: settings.shift_every.min(1440),
    };
    send(&app, Input::DisplaySettings(settings));
}

/// The LEDs at `percent` while the slider moves, before it is saved
#[tauri::command]
pub fn preview_led_brightness(app: AppHandle, percent: u8) {
    send(&app, Input::LedBrightnessPreview(percent.clamp(10, 100)));
}

#[tauri::command]
pub fn preview_overlay(app: AppHandle) {
    send(&app, Input::OverlayPreview);
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
