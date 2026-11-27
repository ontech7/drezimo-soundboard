//! Drezimo's display on the PC screen, for Diagnostics: the picture the app sends the display, each of its dots a whole
//! number of pixels, with the latest keys, pads and knob steps below it, in a window like the overlay's: above the
//! others, letting the mouse through (docs/project.md, section 8). Off at each start: it is never saved.

use super::draw::{self, BACKGROUND, BACKGROUND_ALPHA, Image, LINE, MUTED, TEXT};
use super::native::NativeWindow;
use super::text::{Face, TextStyle};
use super::{Rect, anchored, free_area, run_on};
use crate::display::texts::Texts;
use crate::display::{HEIGHT, WIDTH};
use crate::settings::{Anchor, OverlayScreen};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};
use tauri::AppHandle;

// Sizes in points, multiplied by the screen's scale
const PADDING: f32 = 8.0;
const RADIUS: f32 = 10.0;
const ROW_GAP: f32 = 6.0;
const CHIP_HEIGHT: f32 = 20.0;
const CHIP_PADDING: f32 = 7.0;
const CHIP_GAP: f32 = 4.0;
const CHIP_TEXT: f32 = 11.0;

const GLASS: [u8; 3] = [4, 4, 6];
/// As the window's preview draws the lit dots (src/DisplayPreview.tsx)
const LIT: [u8; 3] = [235, 240, 255];
const CHIP: [u8; 3] = [31, 31, 37];
const CHIP_BORDER: [u8; 3] = [52, 52, 61];
/// A key still held: the window's amber
const HELD: [u8; 3] = [255, 176, 32];
/// A chip not touched for a while
const OLD_ALPHA: f32 = 0.45;
const FADE_AFTER: Duration = Duration::from_secs(4);
/// Knob steps this close in time, the same way, add up in one chip
const TURN_JOIN: Duration = Duration::from_millis(1500);
/// More than fit: the drawing shows the newest that do
const KEPT: usize = 8;
/// Between it and the overlay beside it, in points
const BESIDE_GAP: f64 = 8.0;
/// While shown, how often it looks for its screen again: one may have gone, or changed its scale
const REPLACE_EVERY: Duration = Duration::from_secs(3);

#[derive(Clone, Debug, PartialEq)]
pub struct DisplayLook {
    pub shown: bool,
    pub anchor: Anchor,
    /// The screen's pixels for each of the display's dots, at a 100 % scale
    pub size: u8,
    /// The latest inputs, below the picture
    pub inputs: bool,
    /// At the overlay's place, beside it rather than over it: the overlay's size, 1 for the full one
    pub beside: Option<f64>,
}

impl Default for DisplayLook {
    fn default() -> Self {
        Self {
            shown: false,
            anchor: Anchor::BottomRight,
            size: 2,
            inputs: true,
            beside: None,
        }
    }
}

/// A key, a pad or the knob's button
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Pad(u8),
    Key(u8),
    Knob,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Touch {
    Press(Control, bool),
    /// A knob step: +1 to the right
    Turn(i8),
}

pub enum DisplayUpdate {
    Look(DisplayLook),
    /// The overlay's screen, which the display shows on too
    Screen(OverlayScreen),
    Texts(&'static Texts),
    /// The picture, as the hex of the board's DISPLAY message
    Frame(String),
    /// From the board, or from the window's copy of it when `window`
    Input {
        touch: Touch,
        window: bool,
    },
    /// The board went away: its keys held can no longer be let go
    BoardGone,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Shown {
    Control(Control),
    Turn { right: bool, steps: u32 },
}

#[derive(Clone, Debug, PartialEq)]
struct Chip {
    shown: Shown,
    window: bool,
    held: bool,
    /// Its last press, release or step
    changed: Instant,
}

impl Chip {
    fn old(&self, now: Instant) -> bool {
        !self.held && now.saturating_duration_since(self.changed) >= FADE_AFTER
    }
}

/// The latest inputs, the newest first
#[derive(Default)]
struct Chips(VecDeque<Chip>);

impl Chips {
    fn add(&mut self, touch: Touch, window: bool, now: Instant) {
        match touch {
            Touch::Press(control, true) => self.push(Chip {
                shown: Shown::Control(control),
                window,
                held: true,
                changed: now,
            }),
            Touch::Press(control, false) => {
                let pressed = self
                    .0
                    .iter_mut()
                    .find(|chip| chip.held && chip.window == window && chip.shown == Shown::Control(control));
                if let Some(chip) = pressed {
                    chip.held = false;
                    chip.changed = now;
                }
            }
            Touch::Turn(step) => {
                let right = step > 0;
                if let Some(chip) = self.0.front_mut()
                    && let Shown::Turn { right: same, steps } = &mut chip.shown
                    && *same == right
                    && chip.window == window
                    && now.saturating_duration_since(chip.changed) < TURN_JOIN
                {
                    *steps += 1;
                    chip.changed = now;
                } else {
                    self.push(Chip {
                        shown: Shown::Turn { right, steps: 1 },
                        window,
                        held: false,
                        changed: now,
                    });
                }
            }
        }
    }

    fn board_gone(&mut self, now: Instant) {
        for chip in self.0.iter_mut().filter(|chip| chip.held && !chip.window) {
            chip.held = false;
            chip.changed = now;
        }
    }

    fn push(&mut self, chip: Chip) {
        self.0.push_front(chip);
        self.0.truncate(KEPT);
    }

    /// When a chip next fades, if one will
    fn next_change(&self, now: Instant) -> Option<Instant> {
        self.0
            .iter()
            .filter(|chip| !chip.held && !chip.old(now))
            .map(|chip| chip.changed + FADE_AFTER)
            .min()
    }
}

fn label(shown: Shown, texts: &Texts) -> String {
    match shown {
        Shown::Control(Control::Pad(index)) => format!("R{} C{}", index / 6 + 1, index % 6 + 1),
        Shown::Control(Control::Key(index)) => texts.keys.get(usize::from(index)).copied().unwrap_or("?").to_string(),
        Shown::Control(Control::Knob) => texts.knob.to_string(),
        Shown::Turn { right, steps } => format!("{} {steps}", if right { "→" } else { "←" }),
    }
}

/// The display's dots lit in a frame: 8 bands of 8 rows, 128 columns each, least significant bit on top
fn frame_bytes(hex: &str) -> Option<Vec<u8>> {
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(hex.get(at..at + 2)?, 16).ok())
        .collect::<Option<Vec<u8>>>()?;
    (bytes.len() == (WIDTH * HEIGHT / 8) as usize).then_some(bytes)
}

fn is_lit(frame: &[u8], x: usize, y: usize) -> bool {
    frame[y / 8 * WIDTH as usize + x] & 1 << (y % 8) != 0
}

/// What the panel shows
struct Scene<'a> {
    frame: Option<&'a [u8]>,
    chips: &'a Chips,
    look: &'a DisplayLook,
    texts: &'a Texts,
}

/// What a panel's background depends on: the screen's scale, the dots' size, and whether the inputs show
type Still = (u32, usize, bool);

struct Painter {
    face: Face,
    /// The panel and the glass, the same at each frame
    still: Option<(Still, Image)>,
}

impl Painter {
    /// The panel on a screen of `scale`, each dot `dot` pixels wide.
    fn draw(&mut self, scene: &Scene, scale: f32, dot: usize, now: Instant) -> Image {
        let Scene { frame, look, .. } = *scene;
        let padding = (PADDING * scale).round();
        let (glass_width, glass_height) = (WIDTH as usize * dot, HEIGHT as usize * dot);
        let chip_height = (CHIP_HEIGHT * scale).round();
        let row = if look.inputs {
            (ROW_GAP * scale).round() + chip_height
        } else {
            0.0
        };
        let key = (scale.to_bits(), dot, look.inputs);
        if self.still.as_ref().is_none_or(|(drawn, _)| *drawn != key) {
            let width = glass_width + 2 * padding as usize;
            let height = glass_height + 2 * padding as usize + row as usize;
            let mut still = Image::new(width, height);
            let panel = draw::Rect {
                x: 0.0,
                y: 0.0,
                width: width as f32,
                height: height as f32,
            };
            still.fill_rounded_rect(panel, RADIUS * scale, BACKGROUND, BACKGROUND_ALPHA);
            still.stroke_rounded_rect(panel, RADIUS * scale, scale.round(), LINE, 1.0);
            // Square, as the display's own dots: one lit in a corner stays inside
            still.fill_pixels(padding as usize, padding as usize, glass_width, glass_height, GLASS);
            self.still = Some((key, still));
        }
        let mut image = self
            .still
            .as_ref()
            .map(|(_, still)| still.clone())
            .unwrap_or_else(|| Image::new(0, 0));
        if let Some(frame) = frame {
            for y in 0..HEIGHT as usize {
                for x in 0..WIDTH as usize {
                    if is_lit(frame, x, y) {
                        let (left, top) = (padding as usize + x * dot, padding as usize + y * dot);
                        image.fill_pixels(left, top, dot, dot, LIT);
                    }
                }
            }
        }
        if look.inputs {
            let top = padding + glass_height as f32 + (ROW_GAP * scale).round();
            self.draw_chips(&mut image, scene, (padding, top), glass_width as f32, scale, now);
        }
        image
    }

    /// The chips from (left, top), as many as fit in `room`
    fn draw_chips(
        &mut self,
        image: &mut Image,
        scene: &Scene,
        (left, top): (f32, f32),
        room: f32,
        scale: f32,
        now: Instant,
    ) {
        let Scene { chips, texts, .. } = *scene;
        let px = CHIP_TEXT * scale;
        let height = (CHIP_HEIGHT * scale).round();
        let baseline = top + self.face.baseline(px, height);
        if chips.0.is_empty() {
            let hint = self.face.fit(texts.press_something, px, room);
            let style = TextStyle {
                px,
                color: MUTED,
                alpha: 1.0,
            };
            self.face.draw(image, &hint, left, baseline, style);
            return;
        }
        let padding = (CHIP_PADDING * scale).round();
        let mut x = left;
        for chip in &chips.0 {
            let text = self.face.printable(&label(chip.shown, texts));
            let width = (self.face.width(&text, px) + 2.0 * padding).round();
            if x + width > left + room {
                break;
            }
            let alpha = if chip.old(now) { OLD_ALPHA } else { 1.0 };
            let rect = draw::Rect {
                x,
                y: top,
                width,
                height,
            };
            image.fill_rounded_rect(rect, height / 2.0, CHIP, alpha);
            let border = if chip.held { HELD } else { CHIP_BORDER };
            image.stroke_rounded_rect(rect, height / 2.0, scale.round(), border, alpha);
            let style = TextStyle {
                px,
                color: if chip.window { MUTED } else { TEXT },
                alpha,
            };
            self.face.draw(image, &text, x + padding, baseline, style);
            x += width + (CHIP_GAP * scale).round();
        }
    }
}

thread_local! {
    static WINDOW: RefCell<Option<NativeWindow>> = const { RefCell::new(None) };
}

/// Back in front of the page's overlay, which has just come up. On the main thread.
pub(super) fn raise() {
    WINDOW.with_borrow_mut(|window| {
        if let Some(window) = window {
            window.raise();
        }
    });
}

pub fn start(app: &AppHandle, texts: &'static Texts, screen: OverlayScreen) -> Sender<DisplayUpdate> {
    let (sender, updates) = mpsc::channel();
    let app = app.clone();
    thread::Builder::new()
        .name("display overlay".into())
        .spawn(move || run(&app, &updates, texts, screen))
        .expect("the display overlay's thread starts");
    sender
}

fn run(app: &AppHandle, updates: &Receiver<DisplayUpdate>, mut texts: &'static Texts, mut screen: OverlayScreen) {
    let mut painter = Painter {
        face: Face::medium(),
        still: None,
    };
    let mut look = DisplayLook::default();
    let mut frame: Option<Vec<u8>> = None;
    let mut chips = Chips::default();
    // The screen's free area and scale, found again when the display shows again, the overlay's screen changes, or
    // every few seconds while shown
    let mut area: Option<(Rect, f64)> = None;
    let mut area_found = Instant::now();
    let mut window_shown = false;
    let mut wake: Option<Instant> = None;
    loop {
        let received = match wake {
            Some(at) => updates.recv_timeout(at.saturating_duration_since(Instant::now())),
            None => updates.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        let mut received = match received {
            Ok(update) => vec![update],
            Err(RecvTimeoutError::Timeout) => Vec::new(),
            Err(RecvTimeoutError::Disconnected) => return,
        };
        received.extend(updates.try_iter());
        let now = Instant::now();
        for update in received {
            match update {
                DisplayUpdate::Look(newer) => {
                    // The screens may have changed while it was off
                    if newer.shown && !look.shown {
                        area = None;
                    }
                    look = newer;
                }
                DisplayUpdate::Screen(newer) => {
                    screen = newer;
                    area = None;
                }
                DisplayUpdate::Texts(newer) => texts = newer,
                DisplayUpdate::Frame(hex) => frame = frame_bytes(&hex),
                DisplayUpdate::Input { touch, window } => chips.add(touch, window, now),
                DisplayUpdate::BoardGone => chips.board_gone(now),
            }
        }

        wake = None;
        if !look.shown {
            if window_shown {
                run_on(app, &WINDOW, NativeWindow::hide);
                window_shown = false;
            }
            continue;
        }
        if area.is_none() || now >= area_found + REPLACE_EVERY {
            area = free_area(app, &screen);
            area_found = now;
        }
        let Some((free, scale)) = area else {
            // No screen found yet
            wake = Some(now + super::PLACE_RETRY);
            continue;
        };
        let dot = (f64::from(look.size) * scale).round().max(1.0) as usize;
        let scene = Scene {
            frame: frame.as_deref(),
            chips: &chips,
            look: &look,
            texts,
        };
        let image = painter.draw(&scene, scale as f32, dot, now);
        let (x, y) = placed(free, (image.width, image.height), scale, &look);
        run_on(app, &WINDOW, move |window| {
            window.show(&image, x, y, scale, 1.0);
        });
        window_shown = true;
        let replace = area_found + REPLACE_EVERY;
        wake = Some(chips.next_change(now).map_or(replace, |fade| fade.min(replace)));
    }
}

/// At its place on the screen; beside the overlay when they share it, away from the screen's edge by the overlay's size
fn placed(area: Rect, (width, height): (usize, usize), scale: f64, look: &DisplayLook) -> (i32, i32) {
    let (x, y) = anchored(area, (width, height), scale, look.anchor);
    let Some(overlay) = look.beside else {
        return (x, y);
    };
    // The page's grid, or the action wheel in its place, whichever is bigger
    let page = draw::size((scale * overlay) as f32);
    let wheel = super::wheel::size((scale * overlay) as f32);
    let (overlay_width, overlay_height) = (page.0.max(wheel.0), page.1.max(wheel.1));
    let gap = (BESIDE_GAP * scale).round() as i32;
    let across = overlay_width as i32 + gap;
    let down = overlay_height as i32 + gap;
    match look.anchor {
        Anchor::TopLeft | Anchor::Top | Anchor::TopRight => (x, y + down),
        Anchor::BottomLeft | Anchor::Bottom | Anchor::BottomRight => (x, y - down),
        Anchor::Left => (x + across, y),
        Anchor::Right => (x - across, y),
        // Above the overlay, both centred
        Anchor::Center => (x, y - (overlay_height as i32 + height as i32) / 2 - gap),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::texts::ITALIAN;

    #[test]
    fn a_press_shows_held_until_released() {
        let now = Instant::now();
        let mut chips = Chips::default();
        chips.add(Touch::Press(Control::Pad(13), true), false, now);
        assert!(chips.0[0].held);
        chips.add(Touch::Press(Control::Pad(13), false), true, now);
        assert!(chips.0[0].held, "the window's release is not the board's");
        chips.add(Touch::Press(Control::Pad(13), false), false, now);
        assert!(!chips.0[0].held);
        assert_eq!(label(chips.0[0].shown, &ITALIAN), "R3 C2");
    }

    #[test]
    fn steps_the_same_way_add_up() {
        let now = Instant::now();
        let mut chips = Chips::default();
        for step in 0..3 {
            chips.add(Touch::Turn(1), false, now + Duration::from_millis(200 * step));
        }
        chips.add(Touch::Turn(-1), false, now + Duration::from_millis(700));
        let labels: Vec<String> = chips.0.iter().map(|chip| label(chip.shown, &ITALIAN)).collect();
        assert_eq!(labels, ["← 1", "→ 3"]);

        let later = now + Duration::from_millis(700) + TURN_JOIN;
        chips.add(Touch::Turn(-1), false, later);
        assert_eq!(chips.0.len(), 3, "a pause starts a new chip");
    }

    #[test]
    fn chips_fade_unless_held() {
        let now = Instant::now();
        let mut chips = Chips::default();
        chips.add(Touch::Press(Control::Knob, true), false, now);
        chips.add(Touch::Press(Control::Key(2), true), false, now);
        chips.add(Touch::Press(Control::Key(2), false), false, now);
        assert_eq!(chips.next_change(now), Some(now + FADE_AFTER));
        let later = now + FADE_AFTER;
        assert!(chips.0[0].old(later));
        assert!(!chips.0[1].old(later), "the knob is still held");
        assert_eq!(chips.next_change(later), None);
        assert_eq!(label(chips.0[0].shown, &ITALIAN), "STOP");
    }

    #[test]
    fn a_board_gone_lets_its_keys_go() {
        let now = Instant::now();
        let mut chips = Chips::default();
        chips.add(Touch::Press(Control::Pad(0), true), false, now);
        chips.add(Touch::Press(Control::Knob, true), true, now);
        chips.board_gone(now);
        assert!(chips.0[0].held, "the window still holds its knob");
        assert!(!chips.0[1].held);
    }

    #[test]
    fn keeps_only_the_newest() {
        let now = Instant::now();
        let mut chips = Chips::default();
        for pad in 0..20 {
            chips.add(Touch::Press(Control::Pad(pad), true), false, now);
        }
        assert_eq!(chips.0.len(), KEPT);
        assert_eq!(chips.0[0].shown, Shown::Control(Control::Pad(19)));
    }

    #[test]
    fn draws_each_dot_as_whole_pixels() {
        let mut frame = vec![0; 1024];
        // The dot at (1, 0), and the one at (0, 9)
        frame[1] = 1;
        frame[128] = 1 << 1;
        let look = DisplayLook {
            inputs: false,
            ..DisplayLook::default()
        };
        let mut painter = Painter {
            face: Face::medium(),
            still: None,
        };
        let chips = Chips::default();
        let scene = Scene {
            frame: Some(&frame),
            chips: &chips,
            look: &look,
            texts: &ITALIAN,
        };
        let image = painter.draw(&scene, 1.0, 2, Instant::now());
        assert_eq!((image.width, image.height), (128 * 2 + 16, 64 * 2 + 16));
        let at = |x: usize, y: usize| image.pixel(8 + x, 8 + y);
        assert_eq!(at(2, 0), [235, 240, 255, 255]);
        assert_eq!(at(3, 1), [235, 240, 255, 255]);
        assert_eq!(at(4, 0), [4, 4, 6, 255], "the next dot is dark");
        assert_eq!(at(0, 18), [235, 240, 255, 255]);
    }

    #[test]
    fn the_inputs_add_a_row() {
        let look = DisplayLook::default();
        let mut painter = Painter {
            face: Face::medium(),
            still: None,
        };
        let chips = Chips::default();
        let scene = Scene {
            frame: None,
            chips: &chips,
            look: &look,
            texts: &ITALIAN,
        };
        let image = painter.draw(&scene, 1.5, 3, Instant::now());
        assert_eq!(image.width, 128 * 3 + 2 * 12);
        assert_eq!(image.height, 64 * 3 + 2 * 12 + 9 + 30);
    }

    #[test]
    fn reads_a_frame_and_refuses_a_short_one() {
        assert!(frame_bytes(&"00".repeat(1024)).is_some());
        assert!(frame_bytes(&"00".repeat(1000)).is_none());
        assert!(frame_bytes("zz").is_none());
    }
}
