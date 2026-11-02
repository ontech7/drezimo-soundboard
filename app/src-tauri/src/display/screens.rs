//! The display's screens, laid out as in the simulator.

use super::{Canvas, Size, WIDTH, fit, text_width};

/// The page rows shown at once in the soundboard menu
const MENU_ROWS: usize = 4;
/// The home screen's bottom left
pub const LIVE: &str = "LIVE";

/// A pad's place, such as "R2 C5", from row and column 0.
pub fn position(row: usize, column: usize) -> String {
    format!("R{} C{}", row + 1, column + 1)
}

/// A line of the row overview, such as "3 Airhorn", or "3 -" for an empty pad, from column 0.
pub fn row_line(column: usize, name: Option<&str>) -> String {
    format!("{} {}", column + 1, name.unwrap_or("-"))
}

#[derive(Clone, Debug, PartialEq)]
pub enum Screen {
    /// Voicemod is not connected
    Voicemod { message: String },
    Home {
        title: String,
        counter: String,
        /// The last sound played, or why there is none
        name: String,
        left: String,
        right: String,
    },
    /// The soundboard menu, opened by tapping the knob
    Menu {
        heading: String,
        items: Vec<String>,
        selected: usize,
    },
    /// The name of a pad's sound, shown while the knob is held
    Peek {
        position: String,
        counter: String,
        name: String,
        info: String,
        note: String,
    },
    /// The six sounds of a row, picked by turning the held knob
    Row {
        heading: String,
        counter: String,
        lines: Vec<String>,
    },
}

pub fn draw(screen: &Screen) -> Canvas {
    let mut canvas = Canvas::default();
    match screen {
        Screen::Voicemod { message } => {
            canvas.text_centered("VOICEMOD", 64, 14, Size::Big, true);
            canvas.text_centered(message, 64, 38, Size::Small, true);
        }
        Screen::Home {
            title,
            counter,
            name,
            left,
            right,
        } => {
            let counter_width = text_width(counter, Size::Small);
            canvas.text(
                &fit(title, 127 - counter_width - 4, Size::Small),
                1,
                1,
                Size::Small,
                true,
            );
            canvas.text_right(counter, 127, 1, Size::Small, true);
            canvas.fill_rect(0, 11, WIDTH, 1, true);
            canvas.text_centered(&fit(name, 126, Size::Big), 64, 24, Size::Big, true);
            canvas.text(left, 1, 55, Size::Small, true);
            canvas.text_right(right, 127, 55, Size::Small, true);
        }
        Screen::Menu {
            heading,
            items,
            selected,
        } => {
            canvas.text(heading, 1, 1, Size::Small, true);
            canvas.fill_rect(0, 11, WIDTH, 1, true);
            // The selection stays on the second row, unless the list ends
            let first = selected.saturating_sub(1).min(items.len().saturating_sub(MENU_ROWS));
            for (row, (index, item)) in items.iter().enumerate().skip(first).take(MENU_ROWS).enumerate() {
                let y = 14 + 12 * row as i32;
                let is_selected = index == *selected;
                if is_selected {
                    canvas.fill_rect(0, y - 2, WIDTH, 11, true);
                }
                canvas.text(&fit(item, 124, Size::Small), 2, y, Size::Small, !is_selected);
            }
        }
        Screen::Peek {
            position,
            counter,
            name,
            info,
            note,
        } => {
            inverted_header(&mut canvas, position, counter);
            if text_width(name, Size::Big) <= 126 {
                canvas.text_centered(name, 64, 20, Size::Big, true);
            } else {
                canvas.text_centered(&fit(name, 126, Size::Small), 64, 24, Size::Small, true);
            }
            canvas.text(&fit(info, 126, Size::Small), 1, 45, Size::Small, true);
            canvas.text(note, 1, 55, Size::Small, true);
        }
        Screen::Row {
            heading,
            counter,
            lines,
        } => {
            inverted_header(&mut canvas, heading, counter);
            for (index, line) in lines.iter().enumerate() {
                canvas.text(
                    &fit(line, 126, Size::Small),
                    1,
                    10 + 9 * index as i32,
                    Size::Small,
                    true,
                );
            }
        }
    }
    canvas
}

fn inverted_header(canvas: &mut Canvas, left: &str, right: &str) {
    canvas.fill_rect(0, 0, WIDTH, 9, true);
    canvas.text(left, 1, 1, Size::Small, false);
    canvas.text_right(right, 127, 1, Size::Small, false);
}
