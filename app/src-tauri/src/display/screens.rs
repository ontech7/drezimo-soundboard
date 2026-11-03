//! The display's screens, drawn inside the pixel free on each side that the pixel shift needs. Texts start low enough
//! for their accents to stay inside too.

use super::icons::{self, Icon};
use super::{Canvas, Size, WIDTH, fit, text_width};
use crate::protocol::{COLUMNS, PAD_COUNT};
use std::time::Duration;

/// The page rows shown at once in the soundboard menu
const MENU_ROWS: usize = 4;
/// The widest a line can be, inside the margin
const LINE: i32 = WIDTH - 2;
/// What moves does so at 25 frames a second
pub const FRAME: Duration = Duration::from_millis(40);
/// At each end of a long name, before it scrolls on or starts again
const SCROLL_PAUSE: Duration = Duration::from_millis(1200);
/// On, then off for as long
const BLINK: Duration = Duration::from_millis(500);
/// While Voicemod connects, a dot lights at each step, then they go out
const DOTS_STEP: Duration = Duration::from_millis(330);

/// A pad's place, such as "R2 C5", from row and column 0.
pub fn position(row: usize, column: usize) -> String {
    format!("R{} C{}", row + 1, column + 1)
}

#[derive(Clone, Debug, PartialEq)]
pub enum Screen {
    /// Voicemod is not connected: what to do, or that it is connecting
    Voicemod { message: String, waiting: bool },
    Home {
        title: String,
        counter: String,
        /// The last sound played, the macro running, or why there is none
        name: String,
        progress: Option<Progress>,
        /// The voice loaded, or that the voice changer is off
        voice: Option<String>,
        voice_on: bool,
        /// The switches away from how they usually are, from the right
        switches: Vec<Switch>,
    },
    /// The soundboard menu, opened by tapping the knob
    Menu {
        heading: String,
        items: Vec<String>,
        selected: usize,
        /// The page the board shows
        current: usize,
    },
    /// A pad's content, shown while the knob is held
    Peek {
        position: String,
        counter: String,
        name: String,
        info: String,
        /// On the right of the info: the length, or that it loops
        detail: String,
        note: String,
        /// Outside Voicemod's licence
        locked: bool,
        /// Which of the 36 pads
        pad: usize,
    },
    /// The six pads of a row, picked by turning the held knob
    Row {
        heading: String,
        counter: String,
        names: Vec<Option<String>>,
    },
}

impl Screen {
    /// The name shown big, which scrolls by when too long.
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Home { name, .. } | Self::Peek { name, .. } => Some(name),
            _ => None,
        }
    }

    /// What blinks, or lights up in turn: it starts afresh when this changes.
    pub fn rhythm(&self) -> Option<usize> {
        match self {
            Self::Home {
                progress:
                    Some(Progress::Sound {
                        state: SoundState::Paused,
                        ..
                    }),
                ..
            } => Some(0),
            Self::Home {
                progress: Some(Progress::Macro { step, .. }),
                ..
            } => Some(*step),
            Self::Voicemod { waiting: true, .. } => Some(usize::MAX),
            _ => None,
        }
    }
}

/// Below the home screen's name
#[derive(Clone, Debug, PartialEq)]
pub enum Progress {
    /// The latest sound: how much of it is left, from 1 to 0, when its length is known, and in seconds
    Sound {
        state: SoundState,
        left: Option<f32>,
        time: String,
    },
    /// The macro running, at `step` from 1
    Macro { step: usize, steps: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundState {
    Playing,
    /// Its time blinks
    Paused,
    /// Its bar empties at each round
    Looping,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Switch {
    MicOff,
    MuteForMe,
    HearMe,
}

/// What moves, as of the moment drawn
#[derive(Clone, Copy, Debug, Default)]
pub struct Motion {
    /// How long the big name has been shown
    pub name_for: Duration,
    /// How long what blinks has been on screen
    pub clock: Duration,
}

/// The picture of `screen` as of `motion`, and in how long it changes by itself, if it does.
pub fn draw(screen: &Screen, motion: Motion) -> (Canvas, Option<Duration>) {
    let mut canvas = Canvas::default();
    let changes = match screen {
        Screen::Voicemod { message, waiting } => voicemod(&mut canvas, message, *waiting, motion),
        Screen::Home {
            title,
            counter,
            name,
            progress,
            voice,
            voice_on,
            switches,
        } => {
            header(&mut canvas, title, counter);
            let y = if progress.is_some() { 15 } else { 21 };
            let scrolling = scroll(&mut canvas, name, y, Size::Big, motion.name_for);
            let moving = progress
                .as_ref()
                .and_then(|progress| below_name(&mut canvas, progress, motion));
            strip(&mut canvas, voice.as_deref(), *voice_on, switches);
            soonest(scrolling, moving)
        }
        Screen::Menu {
            heading,
            items,
            selected,
            current,
        } => {
            menu(&mut canvas, heading, items, *selected, *current);
            None
        }
        Screen::Peek {
            position,
            counter,
            name,
            info,
            detail,
            note,
            locked,
            pad,
        } => {
            inverted_header(&mut canvas, position, counter);
            // A name too long for the big letters shows whole in the small ones, if it can: a peek is short
            let scrolling = if text_width(name, Size::Big) > LINE {
                scroll(&mut canvas, name, 19, Size::Small, motion.name_for)
            } else {
                scroll(&mut canvas, name, 14, Size::Big, motion.name_for)
            };
            let left = if *locked {
                canvas.icon(&icons::LOCK, 1, 33, true);
                11
            } else {
                1
            };
            let info_width = LINE - left - text_width(detail, Size::Small) - 4;
            canvas.text(&fit(info, info_width, Size::Small), left, 33, Size::Small, true);
            canvas.text_right(detail, WIDTH - 1, 33, Size::Small, true);
            if !note.is_empty() {
                canvas.icon(&icons::STOP, 1, 46, true);
                canvas.text(&fit(note, 98, Size::Small), 9, 45, Size::Small, true);
            }
            pad_map(&mut canvas, 109, 44, *pad);
            scrolling
        }
        Screen::Row {
            heading,
            counter,
            names,
        } => {
            inverted_header(&mut canvas, heading, counter);
            for (index, name) in names.iter().enumerate().take(COLUMNS) {
                let y = 12 + 8 * index as i32;
                canvas.rounded_rect(1, y, 7, 7, true);
                canvas.text(&(index + 1).to_string(), 1, y, Size::Small, false);
                let name = name.as_deref().unwrap_or("—");
                canvas.text(&fit(name, 114, Size::Small), 11, y, Size::Small, true);
            }
            None
        }
    };
    (canvas, changes)
}

/// The title and the counter, over a line
fn header(canvas: &mut Canvas, title: &str, counter: &str) {
    let counter_width = text_width(counter, Size::Small);
    canvas.text(
        &fit(title, LINE - counter_width - 4, Size::Small),
        1,
        3,
        Size::Small,
        true,
    );
    canvas.text_right(counter, WIDTH - 1, 3, Size::Small, true);
    canvas.fill_rect(1, 11, LINE, 1, true);
}

fn inverted_header(canvas: &mut Canvas, left: &str, right: &str) {
    canvas.rounded_rect(1, 1, LINE, 9, true);
    canvas.text(left, 3, 2, Size::Small, false);
    canvas.text_right(right, WIDTH - 3, 2, Size::Small, false);
}

/// A name, centred, or scrolling by when too long: a pause, then a step at each frame, a pause at the end, and again
/// from the start. Returns when it moves next.
fn scroll(canvas: &mut Canvas, name: &str, y: i32, size: Size, shown_for: Duration) -> Option<Duration> {
    let width = text_width(name, size);
    if width <= LINE {
        canvas.text_centered(name, WIDTH / 2, y, size, true);
        return None;
    }
    // Four letters a second: big letters move two pixels at a time
    let step = match size {
        Size::Small => 1,
        Size::Big => 2,
    };
    // Until its last letter shows, with the space its advance leaves after it
    let travel = width - LINE + 2;
    let steps = (travel + step - 1) / step;
    let scrolling = FRAME * steps as u32;
    let into = phase(shown_for, SCROLL_PAUSE + scrolling + SCROLL_PAUSE);
    let (offset, next) = if into < SCROLL_PAUSE {
        (0, SCROLL_PAUSE - into)
    } else if into < SCROLL_PAUSE + scrolling {
        let moving = into - SCROLL_PAUSE;
        let done = (moving.as_nanos() / FRAME.as_nanos()) as i32;
        ((done * step).min(travel), FRAME - phase(moving, FRAME))
    } else {
        (travel, SCROLL_PAUSE + scrolling + SCROLL_PAUSE - into)
    };
    canvas.text_within(name, 1 - offset, y, size, true, 1..=LINE);
    Some(next)
}

/// The latest sound's state and what is left of it, or a macro's steps. Returns when it moves next.
fn below_name(canvas: &mut Canvas, progress: &Progress, motion: Motion) -> Option<Duration> {
    match progress {
        Progress::Sound { state, left, time } => {
            let icon = match state {
                SoundState::Playing => &icons::PLAY,
                SoundState::Paused => &icons::PAUSE,
                SoundState::Looping => &icons::LOOP,
            };
            canvas.icon(icon, 1, 34, true);
            let time_width = if time.is_empty() {
                0
            } else {
                text_width(time, Size::Small) + 3
            };
            if let Some(left) = left {
                bar(canvas, icon.width() + 4, LINE - time_width, 35, *left);
            }
            let (shown, next) = match state {
                SoundState::Paused if !time.is_empty() => {
                    let (on, next) = blink(motion.clock);
                    (on, Some(next))
                }
                _ => (true, None),
            };
            if shown {
                canvas.text_right(time, WIDTH - 1, 34, Size::Small, true);
            }
            next
        }
        Progress::Macro { step, steps } => {
            canvas.icon(&icons::MACRO, 1, 34, true);
            let counter = format!("{step}/{steps}");
            let right = LINE - text_width(&counter, Size::Small) - 3;
            canvas.text_right(&counter, WIDTH - 1, 34, Size::Small, true);
            let first = icons::MACRO.width() + 5;
            // A square for each step while they fit, else a bar of the steps done
            if first + 6 * *steps as i32 - 3 > right {
                let done = step.saturating_sub(1) as f32 / (*steps).max(1) as f32;
                bar(canvas, first, right, 35, done);
                return None;
            }
            let (on, next) = blink(motion.clock);
            for index in 0..*steps {
                let x = first + 6 * index as i32;
                match (index + 1).cmp(step) {
                    std::cmp::Ordering::Less => canvas.fill_rect(x, 36, 3, 3, true),
                    std::cmp::Ordering::Equal if on => canvas.fill_rect(x, 36, 3, 3, true),
                    std::cmp::Ordering::Equal => canvas.fill_rect(x + 1, 37, 1, 1, true),
                    std::cmp::Ordering::Greater => {
                        canvas.fill_rect(x, 36, 3, 3, true);
                        canvas.fill_rect(x + 1, 37, 1, 1, false);
                    }
                }
            }
            Some(next)
        }
    }
}

/// A dotted track from `left` to `right`, filled from the left for `share` of its length.
fn bar(canvas: &mut Canvas, left: i32, right: i32, y: i32, share: f32) {
    canvas.dotted(left, right, y + 2);
    let length = ((right - left) as f32 * share.clamp(0.0, 1.0)).round() as i32;
    if length > 0 {
        canvas.fill_rect(left, y + 1, length + 1, 3, true);
    }
}

/// The voice on the left, the switches on the right, under a dotted line.
fn strip(canvas: &mut Canvas, voice: Option<&str>, voice_on: bool, switches: &[Switch]) {
    canvas.dotted(1, LINE, 48);
    let mut right = WIDTH - 1;
    for switch in switches {
        let icon = switch_icon(*switch);
        right -= icon.width();
        canvas.icon(icon, right, 53, true);
        right -= 3;
    }
    if let Some(voice) = voice {
        canvas.icon(if voice_on { &icons::WAVE } else { &icons::WAVE_OFF }, 1, 53, true);
        canvas.text(&fit(voice, right - 14, Size::Small), 13, 54, Size::Small, true);
    }
}

fn switch_icon(switch: Switch) -> &'static Icon {
    match switch {
        Switch::MicOff => &icons::MIC_OFF,
        Switch::MuteForMe => &icons::SPEAKER_OFF,
        Switch::HearMe => &icons::HEADPHONES,
    }
}

fn menu(canvas: &mut Canvas, heading: &str, items: &[String], selected: usize, current: usize) {
    let position = if items.is_empty() {
        String::new()
    } else {
        format!("{}/{}", selected + 1, items.len())
    };
    header(canvas, heading, &position);
    // The selection stays on the second row, unless the list ends
    let first = selected.saturating_sub(1).min(items.len().saturating_sub(MENU_ROWS));
    for (row, (index, item)) in items.iter().enumerate().skip(first).take(MENU_ROWS).enumerate() {
        let y = 13 + 13 * row as i32;
        let chosen = index == selected;
        if chosen {
            canvas.rounded_rect(1, y, 121, 11, true);
        }
        canvas.text(&fit(item, 108, Size::Small), 3, y + 2, Size::Small, !chosen);
        if index == current {
            canvas.fill_rect(116, y + 4, 3, 3, !chosen);
        }
    }
    // Where the rows shown are in the list
    if items.len() > MENU_ROWS {
        let (top, track) = (13, 50);
        for y in (top..top + track).step_by(2) {
            canvas.fill_rect(125, y, 1, 1, true);
        }
        let thumb = (track * MENU_ROWS as i32 / items.len() as i32).max(6);
        let scrolled = (track - thumb) * first as i32 / (items.len() - MENU_ROWS) as i32;
        canvas.fill_rect(124, top + scrolled, 3, thumb, true);
    }
}

/// The 36 pads as dots, with `pad` as a square
fn pad_map(canvas: &mut Canvas, x: i32, y: i32, pad: usize) {
    for index in 0..usize::from(PAD_COUNT) {
        let column = x + 1 + 3 * (index % COLUMNS) as i32;
        let row = y + 1 + 3 * (index / COLUMNS) as i32;
        if index == pad {
            canvas.fill_rect(column - 1, row - 1, 3, 3, true);
        } else {
            canvas.fill_rect(column, row, 1, 1, true);
        }
    }
}

fn voicemod(canvas: &mut Canvas, message: &str, waiting: bool, motion: Motion) -> Option<Duration> {
    canvas.icon(&icons::MONITOR, 56, 4, true);
    let next = if waiting {
        let into = phase(motion.clock, DOTS_STEP * 4);
        let lit = (into.as_nanos() / DOTS_STEP.as_nanos()) as i32;
        for dot in 0..lit {
            canvas.fill_rect(59 + 4 * dot, 8, 2, 2, true);
        }
        Some(DOTS_STEP - phase(into, DOTS_STEP))
    } else {
        // An exclamation mark on the screen
        canvas.fill_rect(63, 6, 1, 3, true);
        canvas.fill_rect(63, 10, 1, 1, true);
        None
    };
    canvas.text_centered("VOICEMOD", WIDTH / 2, 21, Size::Big, true);
    canvas.text_centered(&fit(message, LINE, Size::Small), WIDTH / 2, 43, Size::Small, true);
    next
}

/// Whether what blinks is on as of `clock`, and in how long that changes.
fn blink(clock: Duration) -> (bool, Duration) {
    let into = phase(clock, BLINK * 2);
    if into < BLINK {
        (true, BLINK - into)
    } else {
        (false, BLINK * 2 - into)
    }
}

/// How far `time` is into its current round of `period`.
fn phase(time: Duration, period: Duration) -> Duration {
    let nanos = time.as_nanos() % period.as_nanos().max(1);
    Duration::from_nanos(nanos as u64)
}

fn soonest(first: Option<Duration>, second: Option<Duration>) -> Option<Duration> {
    first.into_iter().chain(second).min()
}

#[cfg(test)]
mod tests {
    use super::super::tests::{margin_is_dark, picture};
    use super::*;

    fn home(name: &str, progress: Option<Progress>) -> Screen {
        Screen::Home {
            title: "Stream".into(),
            counter: "1/5".into(),
            name: name.into(),
            progress,
            voice: Some("Robot".into()),
            voice_on: true,
            switches: vec![Switch::MicOff, Switch::MuteForMe],
        }
    }

    fn sound(state: SoundState, left: f32, time: &str) -> Option<Progress> {
        Some(Progress::Sound {
            state,
            left: Some(left),
            time: time.into(),
        })
    }

    fn at(name_for: u64, clock: u64) -> Motion {
        Motion {
            name_for: Duration::from_millis(name_for),
            clock: Duration::from_millis(clock),
        }
    }

    #[test]
    fn keeps_every_screen_inside_the_margin() {
        let off = Screen::Home {
            title: "Città è perché Ça".into(),
            counter: "10/12".into(),
            name: "Perché no?".into(),
            progress: Some(Progress::Macro { step: 10, steps: 30 }),
            voice: Some("VOCE SPENTA".into()),
            voice_on: false,
            switches: vec![Switch::MicOff, Switch::MuteForMe, Switch::HearMe],
        };
        let screens = [
            home("Airhorn", sound(SoundState::Playing, 0.5, "4,2s")),
            home("Applausi dello stadio è così", sound(SoundState::Paused, 0.4, "4,2s")),
            home("Grilli", Some(Progress::Macro { step: 2, steps: 5 })),
            off,
            Screen::Menu {
                heading: "SOUNDBOARD".into(),
                items: (0..12).map(|i| format!("Città {i}")).collect(),
                selected: 11,
                current: 2,
            },
            Screen::Peek {
                position: "R6 C6".into(),
                counter: "1/5".into(),
                name: "Applausi dello stadio è così".into(),
                info: "PLAY/STOP".into(),
                detail: "3,4s".into(),
                note: "FERMA GLI ALTRI".into(),
                locked: true,
                pad: 35,
            },
            Screen::Row {
                heading: "RIGA 6/6".into(),
                counter: "1/5".into(),
                names: vec![Some("È un nome lunghissimo che non finisce mai".into()), None],
            },
            Screen::Voicemod {
                message: "CONNESSIONE".into(),
                waiting: true,
            },
        ];
        for screen in &screens {
            for millis in [0, 600, 1300, 2000, 9000] {
                let (canvas, _) = draw(screen, at(millis, millis));
                assert!(margin_is_dark(&canvas), "{screen:?} at {millis} ms");
            }
        }
    }

    #[test]
    fn a_long_name_scrolls_by_after_a_pause() {
        let screen = home("Applause from the stadium", None);
        let (start, next) = draw(&screen, at(0, 0));
        assert_eq!(next, Some(SCROLL_PAUSE));
        assert_eq!(draw(&screen, at(1000, 0)).0, start);
        let (moved, next) = draw(&screen, at(1240, 0));
        assert_ne!(moved, start);
        assert_eq!(next, Some(FRAME));
        assert_eq!(draw(&screen, at(1250, 0)).1, Some(Duration::from_millis(30)));
    }

    #[test]
    fn a_short_name_stays_still() {
        assert_eq!(draw(&home("Airhorn", None), at(5000, 5000)).1, None);
    }

    #[test]
    fn a_long_peeked_name_shows_whole_in_small_letters() {
        let peek = |name: &str| Screen::Peek {
            position: "R1 C1".into(),
            counter: "1/5".into(),
            name: name.into(),
            info: String::new(),
            detail: String::new(),
            note: String::new(),
            locked: false,
            pad: 0,
        };
        assert_eq!(draw(&peek("Applausi dello stadio"), Motion::default()).1, None);
        assert!(
            draw(&peek("Applausi dello stadio di San Siro"), Motion::default())
                .1
                .is_some()
        );
    }

    #[test]
    fn a_paused_time_blinks() {
        let screen = home("Fanfare", sound(SoundState::Paused, 0.4, "4.2s"));
        let (on, next) = draw(&screen, at(0, 0));
        assert_eq!(next, Some(BLINK));
        let (off, _) = draw(&screen, at(0, 600));
        let time = |canvas: &Canvas| picture(canvas, 106, 35, 21, 5).concat().contains('#');
        assert!(time(&on) && !time(&off));
    }

    #[test]
    fn the_bar_shows_what_is_left() {
        let left = |share: f32| {
            let (canvas, _) = draw(
                &home("Honk", sound(SoundState::Playing, share, "1.0s")),
                Motion::default(),
            );
            (8..90).filter(|x| canvas.is_lit(*x, 36)).count()
        };
        assert!(left(1.0) > left(0.5) && left(0.5) > left(0.0));
        assert_eq!(left(0.0), 0);
    }

    #[test]
    fn no_track_while_what_is_left_is_unknown() {
        let unknown = Some(Progress::Sound {
            state: SoundState::Playing,
            left: None,
            time: String::new(),
        });
        let (canvas, _) = draw(&home("Honk", unknown), Motion::default());
        assert!((8..WIDTH).all(|x| !canvas.is_lit(x, 37)));
    }

    #[test]
    fn shows_each_switch_on_the_right() {
        let (canvas, _) = draw(&home("Ready", None), Motion::default());
        let mut both = Canvas::default();
        both.icon(&icons::SPEAKER_OFF, 104, 53, true);
        both.icon(&icons::MIC_OFF, 116, 53, true);
        assert_eq!(picture(&canvas, 100, 53, 27, 9), picture(&both, 100, 53, 27, 9));
    }

    #[test]
    fn too_many_steps_for_squares_make_a_bar() {
        let (canvas, next) = draw(
            &home("Macro", Some(Progress::Macro { step: 10, steps: 30 })),
            Motion::default(),
        );
        assert_eq!(next, None);
        assert!(canvas.is_lit(14, 36) && canvas.is_lit(14, 38));
    }

    #[test]
    fn marks_the_page_shown_in_the_menu() {
        let menu = |current| {
            let screen = Screen::Menu {
                heading: "SOUNDBOARDS".into(),
                items: vec!["A".into(), "B".into()],
                selected: 0,
                current,
            };
            draw(&screen, Motion::default()).0
        };
        assert!(menu(1).is_lit(117, 31) && !menu(0).is_lit(117, 31));
    }
}
