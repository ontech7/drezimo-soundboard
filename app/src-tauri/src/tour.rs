//! The welcome tour's pictures: the window's sample page on the display and the overlay, drawn as the board and the
//! app draw them, in the language shown.

use crate::display::screens::{self, Screen};
use crate::display::seconds;
use crate::overlay::{self, Cell, Overlay};
use crate::protocol::{Effect, Led};
use crate::settings::SettingsState;
use crate::soundboard::PlaybackMode;
use base64::prelude::{BASE64_STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::State;

const COLUMNS: usize = 6;
const ROWS: usize = 6;
/// Wider, the picture would only be scaled down again
const MAX_OVERLAY_WIDTH: usize = 2400;
/// As a macro has at most
const MAX_STEPS: usize = 50;

/// The page the tour shows, on the board's first page
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sample {
    page: String,
    pages: usize,
    /// The 36 pads, from the top left
    pads: Vec<SamplePad>,
    /// The pad last played, with the seconds its sound has left
    playing: usize,
    left: f64,
    /// The pad peeked, with its sound's length
    peeked: usize,
    length: f64,
    /// The macro tried, which mutes the microphone first
    macro_name: String,
    macro_steps: usize,
    /// The overlay's width in pixels
    overlay_width: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SamplePad {
    /// Empty for a pad without a sound
    name: String,
    color: [u8; 3],
    breathe: bool,
    locked: bool,
}

/// The display's pictures as the hex of their DISPLAY messages, and the overlay's
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pictures {
    home: String,
    peek: String,
    row: String,
    /// While the macro waits, then at each of its steps
    ready: String,
    steps: Vec<String>,
    overlay: Picture,
}

#[derive(Serialize)]
pub struct Picture {
    width: usize,
    height: usize,
    /// Straight RGBA, in base64
    pixels: String,
}

// Async, so the drawing never holds up the main thread
#[tauri::command(async)]
pub fn tour_pictures(settings: State<SettingsState>, sample: Sample) -> Pictures {
    let texts = settings.get().language.texts();
    let counter = format!("1/{}", sample.pages);
    let name = |pad: usize| sample.pads.get(pad).map_or("", |pad| pad.name.as_str());
    let frame = |screen: Screen| screens::draw(&screen).frame().hex();
    let home = |name: &str, right: String| {
        frame(Screen::Home {
            title: sample.page.clone(),
            counter: counter.clone(),
            name: name.to_string(),
            left: screens::LIVE.to_string(),
            right,
        })
    };

    let peek = frame(Screen::Peek {
        position: screens::position(sample.peeked / COLUMNS, sample.peeked % COLUMNS),
        counter: counter.clone(),
        name: name(sample.peeked).to_string(),
        info: format!("{} {}", texts.mode(PlaybackMode::PlayRestart), length(sample.length)),
        note: String::new(),
    });
    let row = frame(Screen::Row {
        heading: texts.row_heading(0, ROWS),
        counter: counter.clone(),
        lines: (0..COLUMNS)
            .map(|column| screens::row_line(column, Some(name(column)).filter(|name| !name.is_empty())))
            .collect(),
    });
    let macro_steps = sample.macro_steps.min(MAX_STEPS);
    let steps = (1..=macro_steps)
        .map(|step| {
            home(
                &sample.macro_name,
                format!("{} {}", texts.mic_off, texts.step_of(step, macro_steps)),
            )
        })
        .collect();

    let grid = Overlay {
        title: sample.page.clone(),
        counter: counter.clone(),
        cells: sample
            .pads
            .iter()
            .take(COLUMNS * ROWS)
            .map(|pad| Cell {
                name: pad.name.clone(),
                led: Led {
                    color: pad.color,
                    effect: if pad.breathe { Effect::Breathe } else { Effect::Solid },
                },
                locked: pad.locked,
            })
            .collect(),
    };
    let (width, height, pixels) = overlay::picture(&grid, sample.overlay_width.clamp(1, MAX_OVERLAY_WIDTH));

    Pictures {
        home: home(name(sample.playing), length(sample.left)),
        peek,
        row,
        ready: home(texts.ready, String::new()),
        steps,
        overlay: Picture {
            width,
            height,
            pixels: BASE64_STANDARD.encode(pixels),
        },
    }
}

/// Seconds as the display shows them, such as "1.5s"
fn length(value: f64) -> String {
    seconds(Duration::try_from_secs_f64(value).unwrap_or_default())
}
