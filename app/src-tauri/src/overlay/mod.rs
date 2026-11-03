//! The 6×6 grid on the PC screen: each pad's sound and LED colour, shown for a few seconds when the page changes and
//! while the knob is held (docs/project.md, section 8). The app draws it into a native window that lets the mouse
//! through and stays above the other windows: a webview would be throttled while hidden on Windows, and slow to appear.

mod draw;
mod text;

#[cfg_attr(target_os = "macos", path = "macos.rs")]
#[cfg_attr(windows, path = "windows.rs")]
mod native;

use crate::protocol::{Effect, Led};
use crate::settings::{Anchor, OverlayScreen};
use draw::Painter;
use native::NativeWindow;
use serde::Serialize;
use std::cell::RefCell;
use std::f32::consts::TAU;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Monitor};

/// The space between the overlay and the edges of the screen's free area, in points
const MARGIN: f64 = 16.0;
const FADE_IN: Duration = Duration::from_millis(120);
const FADE_OUT: Duration = Duration::from_millis(250);
const FADE_FRAME: Duration = Duration::from_millis(16);
/// How often a breathing LED changes
const BREATHE_FRAME: Duration = Duration::from_millis(33);
const BLINK_HALF: u128 = 400;
/// How soon to look for the screen again when it could not be found
const PLACE_RETRY: Duration = Duration::from_millis(250);

#[derive(Clone, Debug, PartialEq)]
pub struct Overlay {
    pub title: String,
    pub counter: String,
    /// The 36 pads, from the top-left
    pub cells: Vec<Cell>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    /// Empty for a pad without a sound
    pub name: String,
    pub led: Led,
    /// Outside the licence: shown dimmed
    pub locked: bool,
}

pub enum OverlayUpdate {
    /// What to show, or None to hide
    Content(Option<Overlay>),
    Look(Look),
}

/// Where the overlay sits, how opaque and how big it is, as set in the window
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    pub anchor: Anchor,
    pub opacity: f32,
    /// 1 for the full size
    pub size: f64,
    pub screen: OverlayScreen,
}

/// A screen, for the window's choice of where the overlay shows
#[derive(Serialize)]
pub struct ScreenView {
    /// As the system names it, with a number added when two screens share a name
    name: String,
    /// Its top-left corner and size, in pixels: the window tells the screens apart by where they are
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    main: bool,
}

/// The screens connected, with their names made unique, and which of them is the main one.
fn connected(app: &AppHandle) -> (Vec<Monitor>, Vec<String>, Option<usize>) {
    let monitors = app.available_monitors().unwrap_or_default();
    let names = unique_names(
        monitors
            .iter()
            .map(|monitor| monitor.name().cloned().unwrap_or_default())
            .collect(),
    );
    let main = app
        .primary_monitor()
        .ok()
        .flatten()
        .and_then(|main| monitors.iter().position(|monitor| same(monitor, &main)));
    (monitors, names, main)
}

fn same(one: &Monitor, other: &Monitor) -> bool {
    one.position() == other.position() && one.size() == other.size()
}

/// Two identical monitors share a name: the second becomes "name (2)".
fn unique_names(names: Vec<String>) -> Vec<String> {
    let mut unique: Vec<String> = Vec::with_capacity(names.len());
    for name in names {
        let mut candidate = name.clone();
        let mut number = 1;
        while unique.contains(&candidate) {
            number += 1;
            candidate = format!("{name} ({number})");
        }
        unique.push(candidate);
    }
    unique
}

/// The screen chosen, by index, or the main one when it cannot be found.
fn pick(chosen: &OverlayScreen, names: &[String], mouse: Option<usize>, main: Option<usize>) -> Option<usize> {
    let found = match chosen {
        OverlayScreen::Mouse => mouse,
        OverlayScreen::Main => None,
        OverlayScreen::Named(name) => names.iter().position(|other| other == name),
    };
    found.or(main)
}

/// A screen's free area, in pixels
#[derive(Clone, Copy, Debug)]
struct Rect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

#[derive(Clone, Copy)]
struct Placement {
    /// The top-left corner, in pixels
    x: i32,
    y: i32,
    /// The screen's
    scale: f64,
    /// The one the overlay is drawn at: smaller when its size is set below full
    drawn: f64,
}

#[derive(Clone, Copy)]
struct Fade {
    from: f32,
    to: f32,
    since: Instant,
}

thread_local! {
    /// The native window, made on the main thread when first needed, and tried again if the system refused it
    static WINDOW: RefCell<Option<NativeWindow>> = const { RefCell::new(None) };
}

static WINDOW_REFUSED: AtomicBool = AtomicBool::new(false);

/// Starts the overlay: it shows each content sent, and hides at `None`.
pub fn start(app: &AppHandle) -> Sender<OverlayUpdate> {
    let (sender, updates) = mpsc::channel();
    let app = app.clone();
    thread::Builder::new()
        .name("overlay".into())
        .spawn(move || run(&app, &updates))
        .expect("the overlay thread starts");
    sender
}

/// The screens connected, as the system names them. Async, as asking the system for them waits on the main thread.
#[tauri::command]
pub async fn screens(app: AppHandle) -> Vec<ScreenView> {
    let (monitors, names, main) = connected(&app);
    monitors
        .iter()
        .zip(names)
        .enumerate()
        .map(|(index, (monitor, name))| ScreenView {
            name,
            x: monitor.position().x,
            y: monitor.position().y,
            width: monitor.size().width,
            height: monitor.size().height,
            main: main == Some(index),
        })
        .collect()
}

/// `overlay` as straight RGBA, `width` pixels wide and each LED at its brightest, for the welcome tour.
pub fn picture(overlay: &Overlay, width: usize) -> (usize, usize, Vec<u8>) {
    let scale = width as f32 / draw::size(1.0).0 as f32;
    let image = Painter::new().draw(overlay, scale, |_| 1.0);
    let mut pixels = image.pixels;
    for pixel in pixels.as_chunks_mut::<4>().0 {
        let alpha = u16::from(pixel[3]);
        for channel in &mut pixel[..3] {
            if let Some(straight) = (u16::from(*channel) * 255 + alpha / 2).checked_div(alpha) {
                *channel = straight.min(255) as u8;
            }
        }
    }
    (image.width, image.height, pixels)
}

fn run(app: &AppHandle, updates: &Receiver<OverlayUpdate>) {
    let clock = Instant::now();
    let mut painter = Painter::new();
    // What the window shows, kept while it fades out
    let mut shown: Option<Overlay> = None;
    let mut visible = false;
    let mut fade = Fade {
        from: 0.0,
        to: 0.0,
        since: clock,
    };
    let mut placement = None;
    let mut look = Look {
        anchor: Anchor::default(),
        opacity: 1.0,
        size: 1.0,
        screen: OverlayScreen::default(),
    };
    let mut redraw = false;
    // The half period of the blinks last drawn, and whether the window shows something
    let mut drawn_blink = None;
    let mut window_shown = false;
    let mut opacity = 0.0;
    let mut next_frame: Option<Instant> = None;
    loop {
        let received = match next_frame {
            Some(at) => updates.recv_timeout(at.saturating_duration_since(Instant::now())),
            None => updates.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        let mut content = None;
        let mut received = match received {
            Ok(update) => vec![update],
            Err(RecvTimeoutError::Timeout) => Vec::new(),
            Err(RecvTimeoutError::Disconnected) => return,
        };
        received.extend(updates.try_iter());
        let now = Instant::now();
        for update in received {
            match update {
                OverlayUpdate::Content(newer) => content = Some(newer),
                OverlayUpdate::Look(newer) if newer != look => {
                    look = newer;
                    // A hidden overlay is placed again when it shows
                    if visible {
                        placement = None;
                        redraw = true;
                        fade = fade.toward(look.opacity, now);
                    }
                }
                OverlayUpdate::Look(_) => {}
            }
        }

        match content {
            Some(Some(overlay)) => {
                if !visible {
                    visible = true;
                    // Placed again: the screens may have changed while hidden
                    placement = None;
                    fade = fade.toward(look.opacity, now);
                }
                shown = Some(overlay);
                redraw = true;
            }
            Some(None) if visible => {
                visible = false;
                fade = fade.toward(0.0, now);
            }
            _ => {}
        }
        if visible && placement.is_none() {
            placement = place(app, &look);
        }

        let shows = |effect| {
            shown
                .iter()
                .flat_map(|overlay| &overlay.cells)
                .any(|cell| cell.led.effect == effect)
        };
        let (breathes, blinks) = (shows(Effect::Breathe), shows(Effect::Blink));
        let time = now - clock;
        let blink = time.as_millis() / BLINK_HALF;
        let new_opacity = fade.opacity(now);
        if let (Some(overlay), Some(at)) = (&shown, placement) {
            if redraw || breathes || (blinks && drawn_blink != Some(blink)) {
                let image = painter.draw(overlay, at.drawn as f32, |cell| level(cell.led.effect, time));
                on_window(app, move |window| {
                    window.show(&image, at.x, at.y, at.scale, new_opacity)
                });
                redraw = false;
                drawn_blink = Some(blink);
                window_shown = true;
            } else if new_opacity != opacity {
                on_window(app, move |window| window.set_opacity(new_opacity));
            }
        }
        opacity = new_opacity;
        if !visible && fade.done(now) && shown.take().is_some() && window_shown {
            on_window(app, NativeWindow::hide);
            window_shown = false;
        }

        let to_flip = BLINK_HALF - time.as_millis() % BLINK_HALF;
        next_frame = if !fade.done(now) {
            Some(now + FADE_FRAME)
        } else if shown.is_none() {
            None
        } else if placement.is_none() {
            Some(now + PLACE_RETRY)
        } else if breathes {
            Some(now + BREATHE_FRAME)
        } else if blinks {
            Some(now + Duration::from_millis(to_flip as u64))
        } else {
            None
        };
    }
}

/// Runs `action` on the native window, on the main thread as the system requires.
fn on_window(app: &AppHandle, action: impl FnOnce(&mut NativeWindow) + Send + 'static) {
    let _ = app.run_on_main_thread(move || {
        WINDOW.with_borrow_mut(|window| {
            if window.is_none() {
                *window = NativeWindow::new();
                if window.is_none() && !WINDOW_REFUSED.swap(true, Ordering::Relaxed) {
                    crate::errors::log("the system refused the overlay's window: trying again at each change");
                }
            }
            if let Some(window) = window {
                action(window);
            }
        });
    });
}

/// Where the overlay goes on the screen chosen, clear of the taskbar or the Dock, and at what size.
fn place(app: &AppHandle, look: &Look) -> Option<Placement> {
    let monitor = screen(app, &look.screen)?;
    let area = monitor.work_area();
    let area = Rect {
        x: area.position.x,
        y: area.position.y,
        width: area.size.width,
        height: area.size.height,
    };
    Some(placement(area, monitor.scale_factor(), look))
}

/// The screen chosen, or the main one when it cannot be found.
fn screen(app: &AppHandle, chosen: &OverlayScreen) -> Option<Monitor> {
    let (monitors, names, main) = connected(app);
    let mouse = matches!(chosen, OverlayScreen::Mouse)
        .then(|| mouse_screen(app))
        .flatten()
        .and_then(|found| monitors.iter().position(|monitor| same(monitor, &found)));
    match pick(chosen, &names, mouse, main) {
        Some(index) => monitors.into_iter().nth(index),
        None => app.primary_monitor().ok().flatten(),
    }
}

fn mouse_screen(app: &AppHandle) -> Option<Monitor> {
    let mouse = app.cursor_position().ok()?;
    // macOS gives the mouse in the main screen's pixels, but looks for the screen in points
    #[cfg(target_os = "macos")]
    let mouse = mouse.to_logical::<f64>(app.primary_monitor().ok()??.scale_factor());
    app.monitor_from_point(mouse.x, mouse.y).ok()?
}

fn placement(area: Rect, scale: f64, look: &Look) -> Placement {
    let drawn = scale * look.size;
    let (width, height) = draw::size(drawn as f32);
    let margin = (MARGIN * scale).round() as i32;
    // 0 at the start of the area, 1 in its middle, 2 at its end
    let (column, row) = match look.anchor {
        Anchor::TopLeft => (0, 0),
        Anchor::Top => (1, 0),
        Anchor::TopRight => (2, 0),
        Anchor::Left => (0, 1),
        Anchor::Center => (1, 1),
        Anchor::Right => (2, 1),
        Anchor::BottomLeft => (0, 2),
        Anchor::Bottom => (1, 2),
        Anchor::BottomRight => (2, 2),
    };
    let along = |start: i32, room: u32, size: usize, step: i32| match step {
        0 => start + margin,
        1 => start + (room as i32 - size as i32) / 2,
        _ => start + room as i32 - size as i32 - margin,
    };
    Placement {
        x: along(area.x, area.width, width, column),
        y: along(area.y, area.height, height, row),
        scale,
        drawn,
    }
}

impl Fade {
    fn length(&self) -> Duration {
        if self.to > self.from { FADE_IN } else { FADE_OUT }
    }

    fn opacity(&self, now: Instant) -> f32 {
        if self.done(now) {
            return self.to;
        }
        let progress = (now - self.since).as_secs_f32() / self.length().as_secs_f32();
        self.from + (self.to - self.from) * progress
    }

    fn toward(self, to: f32, now: Instant) -> Self {
        Self {
            from: self.opacity(now),
            to,
            since: now,
        }
    }

    fn done(&self, now: Instant) -> bool {
        now - self.since >= self.length()
    }
}

/// How bright an LED is at `time`, as the simulator animates its effects.
fn level(effect: Effect, time: Duration) -> f32 {
    let ms = time.as_millis();
    match effect {
        Effect::Solid => 1.0,
        Effect::Breathe => 0.25 + 0.75 * (0.5 + 0.5 * ((ms % 1400) as f32 / 1400.0 * TAU).cos()),
        Effect::Blink if ms % (2 * BLINK_HALF) >= BLINK_HALF => 1.0,
        Effect::Blink => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::OverlayScreen;

    const ANCHORS: [Anchor; 9] = [
        Anchor::TopLeft,
        Anchor::Top,
        Anchor::TopRight,
        Anchor::Left,
        Anchor::Center,
        Anchor::Right,
        Anchor::BottomLeft,
        Anchor::Bottom,
        Anchor::BottomRight,
    ];

    fn look(anchor: Anchor, size: f64) -> Look {
        Look {
            anchor,
            opacity: 1.0,
            size,
            screen: OverlayScreen::Mouse,
        }
    }

    #[test]
    fn names_identical_screens_apart() {
        let names = unique_names(["A", "B", "A", "A"].map(String::from).to_vec());
        assert_eq!(names, ["A", "B", "A (2)", "A (3)"]);
    }

    #[test]
    fn a_named_screen_gone_gives_way_to_the_main_one() {
        let names = ["main", "left"].map(String::from);
        let named = |name: &str| OverlayScreen::Named(name.into());
        assert_eq!(pick(&named("left"), &names, None, Some(0)), Some(1));
        assert_eq!(pick(&named("unplugged"), &names, None, Some(0)), Some(0));
        assert_eq!(pick(&OverlayScreen::Mouse, &names, Some(1), Some(0)), Some(1));
        assert_eq!(pick(&OverlayScreen::Mouse, &names, None, Some(0)), Some(0));
        assert_eq!(pick(&OverlayScreen::Main, &names, Some(1), Some(0)), Some(0));
    }

    #[test]
    fn places_on_a_screen_left_of_the_main_one() {
        let area = Rect {
            x: -1920,
            y: 0,
            width: 1920,
            height: 1040,
        };
        for anchor in ANCHORS {
            let at = placement(area, 1.0, &look(anchor, 1.0));
            let (width, height) = draw::size(at.drawn as f32);
            assert!(at.x >= -1920 && at.x + width as i32 <= 0, "{anchor:?} at x {}", at.x);
            assert!(at.y >= 0 && at.y + height as i32 <= 1040, "{anchor:?} at y {}", at.y);
        }
    }

    #[test]
    fn sizes_with_the_screens_own_scale() {
        let area = Rect {
            x: 2560,
            y: 100,
            width: 2880,
            height: 1620,
        };
        let at = placement(area, 1.5, &look(Anchor::TopLeft, 0.7));
        assert!((at.drawn - 1.05).abs() < 1e-9);
        assert_eq!((at.x, at.y), (2560 + 24, 100 + 24));
        assert_eq!(at.scale, 1.5);
    }

    #[test]
    fn animates_the_effects_as_the_simulator() {
        let at = Duration::from_millis;
        assert_eq!(level(Effect::Solid, at(123)), 1.0);
        assert_eq!(level(Effect::Breathe, at(0)), 1.0);
        assert!((level(Effect::Breathe, at(700)) - 0.25).abs() < 1e-6);
        assert_eq!(level(Effect::Blink, at(100)), 0.0);
        assert_eq!(level(Effect::Blink, at(500)), 1.0);
    }

    #[test]
    fn fades_in_and_out() {
        let start = Instant::now();
        let fade = Fade {
            from: 0.0,
            to: 0.0,
            since: start,
        }
        .toward(1.0, start);
        assert!(fade.opacity(start + FADE_IN / 2) > 0.4 && !fade.done(start + FADE_IN / 2));
        assert!(fade.done(start + FADE_IN));

        let fade = fade.toward(0.0, start + FADE_IN);
        assert_eq!(fade.opacity(start + FADE_IN), 1.0);
        assert!(fade.done(start + FADE_IN + FADE_OUT));
    }

    #[test]
    fn fades_to_a_set_opacity_and_stops() {
        let start = Instant::now();
        let fade = Fade {
            from: 0.95,
            to: 0.95,
            since: start,
        }
        .toward(0.4, start);
        assert!(!fade.done(start + FADE_OUT / 2));
        assert!(fade.done(start + FADE_OUT));
        assert_eq!(fade.opacity(start + FADE_OUT), 0.4);
    }
}
