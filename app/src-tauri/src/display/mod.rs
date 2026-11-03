//! The board's 128×64 display, drawn by the app frame by frame (docs/project.md, section 8).

mod animation;
mod font;
#[rustfmt::skip]
pub mod icons;
pub mod screens;
pub mod texts;

pub use animation::Animator;
use icons::Icon;
use std::fmt::Write;
use std::ops::RangeInclusive;

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
#[derive(Clone, Debug, PartialEq, Eq)]
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
        self.text_within(text, x, y, size, lit, 0..=WIDTH - 1);
    }

    /// Draws `text`, leaving out what falls outside `columns`, as for a name scrolling by.
    pub fn text_within(&mut self, text: &str, x: i32, y: i32, size: Size, lit: bool, columns: RangeInclusive<i32>) {
        let font = size.font();
        let mut left = x;
        for ch in text.chars() {
            let (advance, rows) = font.glyph(ch);
            for (row, bits) in rows.iter().enumerate().take(font.height) {
                for column in 0..32 {
                    if bits & 1 << column != 0 && columns.contains(&(left + column)) {
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

    /// Every other pixel of row `y`, from `left` to `right`.
    pub fn dotted(&mut self, left: i32, right: i32, y: i32) {
        for x in (left..=right).step_by(2) {
            self.set(x, y, true);
        }
    }

    /// Filled, with its corner pixels left as they were.
    pub fn rounded_rect(&mut self, x: i32, y: i32, width: i32, height: i32, lit: bool) {
        let corners = [
            (x, y),
            (x + width - 1, y),
            (x, y + height - 1),
            (x + width - 1, y + height - 1),
        ];
        let kept = corners.map(|(column, row)| self.is_lit(column, row));
        self.fill_rect(x, y, width, height, lit);
        for ((column, row), was) in corners.into_iter().zip(kept) {
            self.set(column, row, was);
        }
    }

    pub fn icon(&mut self, icon: &Icon, x: i32, y: i32, lit: bool) {
        for (column, row) in icon.pixels() {
            self.set(x + column, y + row, lit);
        }
    }

    /// The top `rows` slid sideways from `from` to this picture, `offset` pixels in: forward, `from` leaves on the
    /// left and this comes in from the right.
    pub fn slide_from(&mut self, from: &Canvas, offset: i32, forward: bool, rows: usize) {
        let offset = offset.clamp(0, WIDTH) as u32;
        let rest = WIDTH as u32 - offset;
        // What slides over the margin stays dark
        let inside = !(1 | 1 << (WIDTH - 1));
        for (row, old) in self.rows.iter_mut().zip(from.rows).take(rows) {
            let (leaving, coming) = if forward {
                (old.checked_shr(offset), row.checked_shl(rest))
            } else {
                (old.checked_shl(offset), row.checked_shr(rest))
            };
            *row = (leaving.unwrap_or(0) | coming.unwrap_or(0)) & inside;
        }
    }

    pub fn is_lit(&self, x: i32, y: i32) -> bool {
        (0..WIDTH).contains(&x) && (0..HEIGHT).contains(&y) && self.rows[y as usize] & 1 << x != 0
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

    /// Whether the pixel the pixel shift may cut off on each side is dark.
    pub fn margin_is_dark(canvas: &Canvas) -> bool {
        let edges = (0..WIDTH).flat_map(|x| [(x, 0), (x, HEIGHT - 1)]);
        let sides = (0..HEIGHT).flat_map(|y| [(0, y), (WIDTH - 1, y)]);
        edges.chain(sides).all(|(x, y)| !canvas.is_lit(x, y))
    }

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
