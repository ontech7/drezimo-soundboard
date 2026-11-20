//! Brings the board and Voicemod together: the controller runs on its own thread, gets their events and sends them
//! its messages, and the app's status goes to the tray and the window.

use crate::board::{Board, BoardEvent, BoardStatus};
use crate::board_test::{BoardTest, DisplayTest, LedTest, TEST_BRIGHTNESS, Tester};
use crate::controller::MacroProgress;
use crate::controller::{BoardView, Controller, MutedBy, Output, PageSummary};
use crate::custom_pages::{CustomPage, CustomPagesState, Library, LibraryState};
use crate::firmware::{self, FirmwareUpdate, Updater};
use crate::last_page;
use crate::macros::{Macro, MacrosState};
use crate::overlay::display::{Control, DisplayLook, DisplayUpdate, Touch};
use crate::overlay::{self, Look, OverlayUpdate};
use crate::pictures::Pictures;
use crate::protocol::{self, BUTTON_COUNT, BoardMessage, DisplaySettings, PAD_COUNT};
use crate::settings::{OverlaySettings, Settings, SettingsState};
use crate::tray;
use crate::voicemod::{License, Voicemod, VoicemodEvent, VoicemodStatus};
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
    /// Shows the overlay, as it would look with these settings if given
    OverlayPreview(Option<OverlaySettings>),
    /// The board played from the window
    Pad {
        index: u8,
        pressed: bool,
    },
    Key(u8),
    /// The notice's Hear them again
    HearAgain,
    /// The notice of the volume at zero closed
    CloseVolumeNotice,
    /// The knob turned in the window: +1 to the right
    KnobTurn(i8),
    KnobPress(bool),
    Page(usize),
    CustomPages(Vec<CustomPage>),
    Macros(Vec<Macro>),
    /// The Macro section's Try and Stop
    TryMacro(String),
    StopMacro,
    Test(TestInput),
    /// The display on the PC screen, as Diagnostics sets it
    DisplayLook(DisplayLook),
}

enum TestInput {
    Start,
    Leds(LedTest),
    Display(DisplayTest),
    Restart,
    Stop,
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
    /// Voicemod sent its soundboards: with no page, there is none to show
    pub loaded: bool,
    pub firmware_update: Option<FirmwareUpdate>,
    /// As the connected board keeps them
    pub display_settings: Option<DisplaySettings>,
    /// Voicemod's, while connected
    pub license: Option<License>,
    /// Voicemod's own version, while connected
    pub voicemod_version: Option<String>,
    /// The macro running, if any
    pub running_macro: Option<MacroProgress>,
    /// Why a sound that played was not heard by the user, while it is still so
    pub muted_play: Option<MutedBy>,
    /// Voicemod's bridge was found: a pad can stop a single sound
    pub single_sound_stop: bool,
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

/// The board as the window shows it, when there is something to play
#[derive(Default)]
pub struct BoardViewState(Mutex<Option<BoardView>>);

#[tauri::command]
pub fn board_view(view: State<BoardViewState>) -> Option<BoardView> {
    view.0.lock().unwrap().clone()
}

#[tauri::command]
pub fn display_frame(frame: State<DisplayFrame>) -> String {
    frame.0.lock().unwrap().clone()
}

/// The display on the PC screen, as Diagnostics shows it: off at each start
#[derive(Default)]
pub struct DisplayLookState(Mutex<DisplayLook>);

#[tauri::command]
pub fn display_look(look: State<DisplayLookState>) -> DisplayLook {
    look.0.lock().unwrap().clone()
}

#[tauri::command]
pub fn set_display_look(app: AppHandle, look: DisplayLook) {
    send(&app, Input::DisplayLook(look.checked()));
}

/// The board test, while one runs
#[derive(Default)]
pub struct BoardTestState(Mutex<Option<BoardTest>>);

#[tauri::command]
pub fn board_test(test: State<BoardTestState>) -> Option<BoardTest> {
    test.0.lock().unwrap().clone()
}

/// Starts looking for the board and Voicemod. Expects `Mutex<Status>`, `DisplayFrame`, `BoardViewState`,
/// `BoardTestState`, `DisplayLookState`, `LastUse`, `SettingsState`, `CustomPagesState`, `MacrosState`, `LibraryState` and `Pictures` to
/// be managed.
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
    let pictures_app = app.clone();
    let voicemod = Voicemod::start(move |event| {
        let pictures = pictures_app.state::<Pictures>();
        match event {
            VoicemodEvent::Picture { of, file } => pictures.received(of, file),
            event => {
                if event == VoicemodEvent::Status(VoicemodStatus::Connected) {
                    pictures.reconnected();
                }
                let _ = inputs.send(Input::Voicemod(event));
            }
        }
    });
    app.state::<Pictures>().connect(voicemod.clone());
    let overlay = overlay::start(&app);
    let settings = app.state::<SettingsState>().get();
    let display = overlay::display::start(&app, settings.language.texts(), settings.overlay.screen);
    thread::spawn(move || {
        let outputs = Outputs {
            board,
            voicemod,
            overlay,
            display,
        };
        run(&app, &received, &firmware_inputs, &outputs);
    });
}

/// Where the controller's work goes
struct Outputs {
    board: Board,
    voicemod: Voicemod,
    overlay: Sender<OverlayUpdate>,
    /// The display on the PC screen
    display: Sender<DisplayUpdate>,
}

fn run(app: &AppHandle, inputs: &Receiver<Input>, firmware_inputs: &Sender<Input>, outputs: &Outputs) {
    let Outputs {
        board,
        voicemod,
        overlay,
        display,
    } = outputs;
    let mut settings = app.state::<SettingsState>().get();
    let mut controller = Controller::new(settings.language.texts());
    controller.set_press_again(settings.press_again_by_mode);
    controller.set_knob(settings.knob.clone(), Instant::now());
    controller.set_custom_pages(app.state::<CustomPagesState>().get());
    controller.set_macros(app.state::<MacrosState>().get());
    let mut kept_page = last_page::load(app);
    if let Some(page) = kept_page.clone() {
        controller.start_on(page);
    }
    let mut shown_lengths = None;
    controller.set_overlay_time(overlay_time(&settings.overlay));
    let _ = overlay.send(OverlayUpdate::Look(look(&settings.overlay)));
    let mut status = Status::default();
    let mut published = status.clone();
    let mut updater = Updater::default();
    // SETTINGS sent and not answered yet: only the answer to the last one is current
    let mut settings_reads = 0;
    let mut shown_view = None;
    // While a test runs, the controller's display waits here
    let mut screen = String::new();
    let mut tester: Option<Tester> = None;
    let mut shown_test = None;
    let mut shown_picture = None;
    loop {
        let wake = controller
            .next_update()
            .into_iter()
            .chain(updater.next_update())
            .chain(tester.as_ref().and_then(Tester::next_update))
            .min();
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
        // A test that ends gives the board back: no new connection to show the overlay for
        let mut test_ended = false;
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
                    board.send(brightness_message(led_brightness(&settings, tester.is_some())));
                    board.send("SETTINGS");
                    settings_reads = 1;
                    // Development aid: the module's own LED copies pad 1
                    #[cfg(debug_assertions)]
                    board.send("DEVLED,MIRROR,0");
                }
                if !matches!(board_status, BoardStatus::Connected { .. }) {
                    status.display_settings = None;
                    let _ = display.send(DisplayUpdate::BoardGone);
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
                if let Some(touch) = touch(&message) {
                    *app.state::<LastUse>().0.lock().unwrap() = Some(now);
                    let _ = display.send(DisplayUpdate::Input { touch, window: false });
                }
                match &mut tester {
                    Some(tester) => tester.board_message(&message),
                    None => controller.board_message(message, now),
                }
            }
            Some(Input::Voicemod(event)) => {
                match &event {
                    VoicemodEvent::Status(voicemod_status) => {
                        status.voicemod = *voicemod_status;
                        status.license = None;
                        status.voicemod_version = None;
                    }
                    VoicemodEvent::License(license) => status.license = Some(*license),
                    VoicemodEvent::Version(version) => status.voicemod_version = Some(version.clone()),
                    VoicemodEvent::SingleSoundStop(there) => status.single_sound_stop = *there,
                    VoicemodEvent::Soundboards(soundboards) => {
                        update_library(app, |library| library.set_soundboards(soundboards));
                    }
                    VoicemodEvent::Voices { voices, .. } => update_library(app, |library| library.set_voices(voices)),
                    _ => {}
                }
                controller.voicemod_event(event, now);
            }
            Some(Input::Log(input)) => controller.log_input(input, now),
            Some(Input::Firmware(result)) => {
                if let Err(reason) = &result {
                    crate::errors::log(format!("the board's firmware update failed: {reason}"));
                }
                updater.installed(result, now);
            }
            Some(Input::Settings(new)) => {
                controller.set_texts(new.language.texts());
                controller.set_press_again(new.press_again_by_mode);
                controller.set_knob(new.knob.clone(), Instant::now());
                controller.set_overlay_time(overlay_time(&new.overlay));
                let _ = overlay.send(OverlayUpdate::Look(look(&new.overlay)));
                let _ = display.send(DisplayUpdate::Texts(new.language.texts()));
                let _ = display.send(DisplayUpdate::Screen(new.overlay.screen.clone()));
                if !new.diagnostics {
                    show_display(app, display, DisplayLook::default());
                }
                if new.led_brightness != settings.led_brightness {
                    board.send(brightness_message(led_brightness(&new, tester.is_some())));
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
            Some(Input::OverlayPreview(looking)) => {
                if let Some(overlay_settings) = looking {
                    let _ = overlay.send(OverlayUpdate::Look(look(&overlay_settings)));
                }
                controller.preview_overlay(now);
            }
            Some(Input::Pad { index, pressed }) => {
                controller.window_pad(index, pressed, now);
                window_touch(display, Touch::Press(Control::Pad(index), pressed));
            }
            Some(Input::Key(index)) => {
                controller.window_key(index, now);
                // A click: pressed and let go at once
                window_touch(display, Touch::Press(Control::Key(index), true));
                window_touch(display, Touch::Press(Control::Key(index), false));
            }
            Some(Input::HearAgain) => controller.hear_sounds_again(),
            Some(Input::CloseVolumeNotice) => controller.close_volume_notice(),
            Some(Input::KnobTurn(step)) => {
                controller.window_knob_turned(step, now);
                window_touch(display, Touch::Turn(step));
            }
            Some(Input::KnobPress(pressed)) => {
                controller.window_knob_pressed(pressed, now);
                window_touch(display, Touch::Press(Control::Knob, pressed));
            }
            Some(Input::Page(index)) => controller.go_to_page(index),
            Some(Input::CustomPages(pages)) => controller.set_custom_pages(pages),
            Some(Input::Macros(macros)) => controller.set_macros(macros),
            Some(Input::TryMacro(id)) => controller.try_macro(&id, now),
            Some(Input::StopMacro) => controller.stop_macro(),
            Some(Input::Test(input)) => match (input, &mut tester) {
                (TestInput::Start, None) => {
                    // Nothing plays meanwhile, a macro tried on the Macros page included
                    controller.stop_macro();
                    board.send(brightness_message(led_brightness(&settings, true)));
                    tester = Some(Tester::default());
                }
                (TestInput::Leds(leds), Some(tester)) => tester.set_leds(leds),
                (TestInput::Display(display), Some(tester)) => tester.set_display(display),
                (TestInput::Restart, Some(tester)) => tester.restart(),
                (TestInput::Stop, Some(_)) => {
                    board.send(brightness_message(settings.led_brightness));
                    tester = None;
                    test_ended = true;
                }
                _ => {}
            },
            Some(Input::DisplayLook(look)) => show_display(app, display, look),
            None => {}
        }
        updater.update(now);
        // A board being updated restarts in its bootloader: nothing to drive meanwhile
        let updating = updater.state().as_ref().is_some_and(FirmwareUpdate::in_progress);
        let connected = matches!(status.board, BoardStatus::Connected { .. }) && !updating;
        // The test has the board meanwhile
        let driven = connected && tester.is_none();
        controller.set_board_connected(driven, now);
        if test_ended {
            controller.hide_overlay();
        }

        controller.update(now);
        for output in controller.take_outputs() {
            match output {
                Output::Board(message) => board.send(message),
                Output::Voicemod(request) => voicemod.send(request),
                Output::Display(frame) => {
                    screen = frame.hex();
                    if driven {
                        board.send(protocol::display_message(&screen));
                    }
                    if tester.is_none() {
                        show_frame(app, display, screen.clone());
                    }
                }
                Output::Overlay(shown) => {
                    let _ = overlay.send(OverlayUpdate::Content(shown));
                }
                Output::Log(message) => crate::errors::log(message),
                Output::Gesture(gesture) => {
                    let _ = app.emit("knob-gesture", gesture);
                }
            }
        }
        match &mut tester {
            Some(tester) if connected => {
                for message in tester.messages(now) {
                    board.send(message);
                }
            }
            Some(tester) => tester.disconnected(),
            None => {}
        }
        let picture = tester.as_ref().map(|tester| tester.test().display);
        if picture != shown_picture {
            show_frame(
                app,
                display,
                tester
                    .as_ref()
                    .map_or_else(|| screen.clone(), |tester| tester.picture().hex()),
            );
            shown_picture = picture;
        }
        if tester.as_ref().map(Tester::test) != shown_test.as_ref() {
            shown_test = tester.as_ref().map(|tester| tester.test().clone());
            // Stored before it is announced, as the status is
            app.state::<BoardTestState>().0.lock().unwrap().clone_from(&shown_test);
            let _ = app.emit("board-test", &shown_test);
        }
        let view = controller.board_view(now);
        if view != shown_view {
            // Stored before it is announced, as the status is
            app.state::<BoardViewState>().0.lock().unwrap().clone_from(&view);
            let _ = app.emit("board-view", &view);
            shown_view = view;
        }
        // Learnt as sounds play: the Macro section times its steps with them
        if shown_lengths != Some(controller.lengths_learned()) {
            shown_lengths = Some(controller.lengths_learned());
            update_library(app, |library| library.set_lengths(controller.sound_lengths()));
        }
        if let Some(page) = controller.shown_page().filter(|page| kept_page.as_ref() != Some(page)) {
            last_page::save(app, &page);
            kept_page = Some(page);
        }
        status.page = controller.page();
        status.loaded = controller.soundboards_loaded();
        status.running_macro = controller.macro_progress();
        status.voicemod_log = controller.follows_log();
        status.muted_play = controller.played_while_muted();
        status.firmware_update = updater.state();
        if status != published {
            publish(app, &status);
            published = status.clone();
        }
    }
}

/// The display's picture, for the window's preview, stored before it is announced, and for the display on the PC
/// screen
fn show_frame(app: &AppHandle, display: &Sender<DisplayUpdate>, hex: String) {
    app.state::<DisplayFrame>().0.lock().unwrap().clone_from(&hex);
    let _ = app.emit("display", &hex);
    let _ = display.send(DisplayUpdate::Frame(hex));
}

/// The display on the PC screen as asked, stored for the window
fn show_display(app: &AppHandle, display: &Sender<DisplayUpdate>, look: DisplayLook) {
    app.state::<DisplayLookState>().0.lock().unwrap().clone_from(&look);
    let _ = display.send(DisplayUpdate::Look(look));
}

fn window_touch(display: &Sender<DisplayUpdate>, touch: Touch) {
    let _ = display.send(DisplayUpdate::Input { touch, window: true });
}

/// A key, a pad or the knob, as the display on the PC screen shows it
fn touch(message: &BoardMessage) -> Option<Touch> {
    match *message {
        BoardMessage::Pad { index, pressed } => Some(Touch::Press(Control::Pad(index), pressed)),
        BoardMessage::Button { index, pressed } => Some(Touch::Press(Control::Key(index), pressed)),
        BoardMessage::Encoder { step } => Some(Touch::Turn(step)),
        BoardMessage::EncoderButton { pressed } => Some(Touch::Press(Control::Knob, pressed)),
        _ => None,
    }
}

/// The editor's library follows Voicemod's lists.
fn update_library(app: &AppHandle, change: impl FnOnce(&mut Library)) {
    let state = app.state::<LibraryState>();
    let library = {
        let mut library = state.0.lock().unwrap();
        change(&mut library);
        library.clone()
    };
    let _ = app.emit("library", &library);
}

fn brightness_message(percent: u8) -> String {
    format!("BRIGHT,{percent}")
}

fn led_brightness(settings: &Settings, testing: bool) -> u8 {
    if testing {
        settings.led_brightness.min(TEST_BRIGHTNESS)
    } else {
        settings.led_brightness
    }
}

fn overlay_time(overlay: &OverlaySettings) -> Option<Duration> {
    overlay.enabled.then(|| Duration::from_secs(overlay.seconds.into()))
}

fn look(overlay: &OverlaySettings) -> Look {
    Look {
        anchor: overlay.anchor,
        opacity: f32::from(overlay.opacity) / 100.0,
        size: f64::from(overlay.size) / 100.0,
        screen: overlay.screen.clone(),
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

pub fn apply_custom_pages(app: &AppHandle, pages: Vec<CustomPage>) {
    send(app, Input::CustomPages(pages));
}

pub fn apply_macros(app: &AppHandle, macros: Vec<Macro>) {
    send(app, Input::Macros(macros));
}

pub fn try_macro(app: &AppHandle, id: String) {
    send(app, Input::TryMacro(id));
}

pub fn stop_macro(app: &AppHandle) {
    send(app, Input::StopMacro);
}

#[tauri::command]
pub fn set_display_settings(app: AppHandle, settings: DisplaySettings) {
    apply_display_settings(&app, settings);
}

/// Within what the firmware takes: minutes up to a day
pub fn apply_display_settings(app: &AppHandle, settings: DisplaySettings) {
    let settings = DisplaySettings {
        brightness: settings.brightness.min(100),
        dim_after: settings.dim_after.min(1440),
        off_after: settings.off_after.min(1440),
        shift_every: settings.shift_every.min(1440),
    };
    send(app, Input::DisplaySettings(settings));
}

/// The connected board's, or none without a board
pub fn display_settings(app: &AppHandle) -> Option<DisplaySettings> {
    app.try_state::<Mutex<Status>>()?.lock().unwrap().display_settings
}

/// The LEDs at `percent` while the slider moves, before it is saved
#[tauri::command]
pub fn preview_led_brightness(app: AppHandle, percent: u8) {
    send(&app, Input::LedBrightnessPreview(percent.clamp(10, 100)));
}

/// The overlay now, as it looks with `overlay` while a slider moves, before it is saved
#[tauri::command]
pub fn preview_overlay(app: AppHandle, overlay: Option<OverlaySettings>) {
    send(&app, Input::OverlayPreview(overlay.map(OverlaySettings::checked)));
}

/// A pad pressed or released in the window
#[tauri::command]
pub fn press_pad(app: AppHandle, index: u8, pressed: bool) {
    if index < PAD_COUNT {
        send(&app, Input::Pad { index, pressed });
    }
}

/// Brings back the sounds the user did not hear, whatever kept them away
#[tauri::command]
pub fn hear_sounds_again(app: AppHandle) {
    send(&app, Input::HearAgain);
}

/// "Fine as it is" on the notice of the soundboard's volume at zero for the user, until the app starts again
#[tauri::command]
pub fn close_volume_notice(app: AppHandle) {
    send(&app, Input::CloseVolumeNotice);
}

/// The knob turned in the window, a step at a time
#[tauri::command]
pub fn turn_knob(app: AppHandle, step: i8) {
    if step != 0 {
        send(&app, Input::KnobTurn(step.signum()));
    }
}

/// The knob pressed or let go in the window
#[tauri::command]
pub fn press_knob(app: AppHandle, pressed: bool) {
    send(&app, Input::KnobPress(pressed));
}

/// A function key clicked in the window
#[tauri::command]
pub fn press_key(app: AppHandle, index: u8) {
    if index < BUTTON_COUNT {
        send(&app, Input::Key(index));
    }
}

#[tauri::command]
pub fn go_to_page(app: AppHandle, index: usize) {
    send(&app, Input::Page(index));
}

/// Takes the board for a test, or goes on with the one running
#[tauri::command]
pub fn start_board_test(app: AppHandle) {
    send(&app, Input::Test(TestInput::Start));
}

#[tauri::command]
pub fn set_board_test_leds(app: AppHandle, leds: LedTest) {
    send(&app, Input::Test(TestInput::Leds(leds)));
}

#[tauri::command]
pub fn set_board_test_display(app: AppHandle, display: DisplayTest) {
    send(&app, Input::Test(TestInput::Display(display)));
}

#[tauri::command]
pub fn restart_board_test(app: AppHandle) {
    send(&app, Input::Test(TestInput::Restart));
}

#[tauri::command]
pub fn stop_board_test(app: AppHandle) {
    end_board_test(&app);
}

/// Gives the board back to the soundboard.
pub fn end_board_test(app: &AppHandle) {
    send(app, Input::Test(TestInput::Stop));
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
