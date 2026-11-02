//! The board's 128×64 display, drawn by the app as the simulator draws it (docs/project.md, section 9).

mod font;
pub mod screens;
pub mod texts;

use std::fmt::Write;
use std::time::Duration;

pub const WIDTH: i32 = 128;
pub const HEIGHT: i32 = 64;
const FRAME_BYTES: usize = (WIDTH * HEIGHT / 8) as usize;

pub struct Font {
    /// Where the glyphs' first row is, from the text's y
    top: i32,
    height: usize,
    /// Sorted by character: the advance, then one bit mask per row
    glyphs: &'static [(char, u8, &'static [u32])],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Size {
    Small,
    Big,
}

impl Size {
    fn font(self) -> &'static Font {
        match self {
            Self::Small => &font::SMALL,
            Self::Big => &font::BIG,
        }
    }
}

impl Font {
    /// The glyph for `ch`, or a question mark for a character the font does not have.
    fn glyph(&self, ch: char) -> (u8, &'static [u32]) {
        let find = |ch: char| {
            let index = self.glyphs.binary_search_by_key(&ch, |(glyph, ..)| *glyph).ok()?;
            let (_, advance, rows) = self.glyphs[index];
            Some((advance, rows))
        };
        find(ch).or_else(|| find('?')).expect("the font has a question mark")
    }
}

pub fn text_width(text: &str, size: Size) -> i32 {
    let font = size.font();
    text.chars().map(|ch| i32::from(font.glyph(ch).0)).sum()
}

/// A duration as the simulator shows it, such as "1.5s".
pub fn seconds(duration: Duration) -> String {
    format!("{:.1}s", duration.as_secs_f64())
}

/// `text`, shortened with an ellipsis to fit `max` pixels.
pub fn fit(text: &str, max: i32, size: Size) -> String {
    if text_width(text, size) <= max {
        return text.to_string();
    }
    let mut shortened: String = text.to_string();
    while !shortened.is_empty() && text_width(&shortened, size) + text_width("…", size) > max {
        shortened.pop();
    }
    shortened + "…"
}

/// A picture for the display, one bit per pixel.
#[derive(Clone, PartialEq, Eq)]
pub struct Canvas {
    rows: [u128; HEIGHT as usize],
}

impl Default for Canvas {
    fn default() -> Self {
        Self {
            rows: [0; HEIGHT as usize],
        }
    }
}

impl Canvas {
    pub fn fill_rect(&mut self, x: i32, y: i32, width: i32, height: i32, lit: bool) {
        for row in y.max(0)..(y + height).min(HEIGHT) {
            for column in x.max(0)..(x + width).min(WIDTH) {
                self.set(column, row, lit);
            }
        }
    }

    /// Draws `text` with its top-left corner at (x, y), lit or dark.
    pub fn text(&mut self, text: &str, x: i32, y: i32, size: Size, lit: bool) {
        let font = size.font();
        let mut left = x;
        for ch in text.chars() {
            let (advance, rows) = font.glyph(ch);
            for (row, bits) in rows.iter().enumerate().take(font.height) {
                for column in 0..32 {
                    if bits & 1 << column != 0 {
                        self.set(left + column, y + font.top + row as i32, lit);
                    }
                }
            }
            left += i32::from(advance);
        }
    }

    pub fn text_centered(&mut self, text: &str, center: i32, y: i32, size: Size, lit: bool) {
        let x = center - text_width(text, size) / 2;
        self.text(text, x, y, size, lit);
    }

    pub fn text_right(&mut self, text: &str, right: i32, y: i32, size: Size, lit: bool) {
        self.text(text, right - text_width(text, size), y, size, lit);
    }

    pub fn is_lit(&self, x: i32, y: i32) -> bool {
        self.rows[y as usize] & 1 << x != 0
    }

    /// The bytes the board takes: 8 bands of 8 rows, 128 columns each, least significant bit on top.
    pub fn frame(&self) -> Frame {
        let mut bytes = [0; FRAME_BYTES];
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                if self.is_lit(x, y) {
                    bytes[(y / 8 * WIDTH + x) as usize] |= 1 << (y % 8);
                }
            }
        }
        Frame(Box::new(bytes))
    }

    fn set(&mut self, x: i32, y: i32, lit: bool) {
        if !(0..WIDTH).contains(&x) || !(0..HEIGHT).contains(&y) {
            return;
        }
        let row = &mut self.rows[y as usize];
        if lit {
            *row |= 1 << x;
        } else {
            *row &= !(1 << x);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame(Box<[u8; FRAME_BYTES]>);

impl Frame {
    pub fn hex(&self) -> String {
        let mut hex = String::with_capacity(FRAME_BYTES * 2);
        for byte in self.0.iter() {
            let _ = write!(hex, "{byte:02X}");
        }
        hex
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The lit pixels of a canvas region, as rows of `#` and `.`.
    pub fn picture(canvas: &Canvas, x: i32, y: i32, width: i32, height: i32) -> Vec<String> {
        (y..y + height)
            .map(|row| {
                let pixel = |column| if canvas.is_lit(column, row) { '#' } else { '.' };
                (x..x + width).map(pixel).collect()
            })
            .collect()
    }

    #[test]
    fn draws_the_simulator_font() {
        let mut canvas = Canvas::default();
        canvas.text("Aè", 0, 2, Size::Small, true);
        let expected = [
            ".......#..", //
            "........#.",
            "..........",
            "..##...###",
            ".#..#..#..",
            ".####..###",
            ".#..#..#..",
            ".#..#..###",
        ];
        assert_eq!(picture(&canvas, 0, 0, 10, 8), expected);
        assert_eq!(text_width("Aè", Size::Small), 11);
    }

    #[test]
    fn fits_text_with_an_ellipsis() {
        assert_eq!(fit("HONK", 100, Size::Small), "HONK");
        let fitted = fit("A VERY LONG SOUND NAME", 40, Size::Small);
        assert!(
            fitted.ends_with('…') && text_width(&fitted, Size::Small) <= 40,
            "{fitted}"
        );
    }

    #[test]
    fn unknown_characters_show_as_a_question_mark() {
        assert_eq!(text_width("🔥", Size::Big), text_width("?", Size::Big));
    }

    #[test]
    fn packs_the_frame_in_bands_of_eight_rows() {
        let mut canvas = Canvas::default();
        canvas.fill_rect(0, 0, 1, 1, true);
        canvas.fill_rect(127, 63, 1, 1, true);
        canvas.fill_rect(5, 9, 1, 1, true);
        let Frame(bytes) = canvas.frame();
        assert_eq!(bytes[0], 0x01);
        assert_eq!(bytes[1023], 0x80);
        assert_eq!(bytes[128 + 5], 0x02);
        assert!(canvas.frame().hex().starts_with("01000000"));
        assert_eq!(canvas.frame().hex().len(), 2048);
    }
}
