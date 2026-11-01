//! The overlay's text: Inter (SIL Open Font License, `OFL.txt` in this folder), subset to Latin, with its kerning
//! moved into the `kern` table that fontdue reads.

use super::draw::Image;
use fontdue::{Font, FontSettings, Metrics};
use std::collections::HashMap;

pub struct Face {
    font: Font,
    glyphs: HashMap<(char, u32), (Metrics, Vec<u8>)>,
}

/// How text is drawn: its size in pixels, its colour and its opacity
#[derive(Clone, Copy)]
pub struct TextStyle {
    pub px: f32,
    pub color: [u8; 3],
    pub alpha: f32,
}

/// A character of a line, with where it starts and ends, kerning included
struct Placed {
    index: usize,
    ch: char,
    start: f32,
    end: f32,
}

impl Face {
    pub fn medium() -> Self {
        Self::new(include_bytes!("Inter-Medium.ttf"))
    }

    pub fn semibold() -> Self {
        Self::new(include_bytes!("Inter-SemiBold.ttf"))
    }

    fn new(bytes: &[u8]) -> Self {
        let font = Font::from_bytes(bytes, FontSettings::default()).expect("the overlay's fonts are valid");
        Self {
            font,
            glyphs: HashMap::new(),
        }
    }

    /// `text` with only the characters the font has, its spaces collapsed.
    pub fn printable(&self, text: &str) -> String {
        let known: String = text
            .chars()
            .map(|ch| if ch.is_whitespace() { ' ' } else { ch })
            .filter(|ch| *ch == ' ' || self.font.lookup_glyph_index(*ch) != 0)
            .collect();
        known.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// The distance from the top of a line to its baseline, for a line `height` pixels high.
    pub fn baseline(&self, px: f32, height: f32) -> f32 {
        let metrics = self
            .font
            .horizontal_line_metrics(px)
            .expect("the overlay's fonts are horizontal");
        let text_height = metrics.ascent - metrics.descent;
        (height - text_height) / 2.0 + metrics.ascent
    }

    pub fn width(&self, text: &str, px: f32) -> f32 {
        self.layout(text, px).last().map_or(0.0, |placed| placed.end)
    }

    /// `text`, shortened with an ellipsis to fit `max_width`.
    pub fn fit(&self, text: &str, px: f32, max_width: f32) -> String {
        if self.width(text, px) <= max_width {
            return text.to_string();
        }
        let room = max_width - self.width("…", px);
        let end = self
            .layout(text, px)
            .take_while(|placed| placed.end <= room)
            .last()
            .map_or(0, |placed| placed.index + placed.ch.len_utf8());
        format!("{}…", text[..end].trim_end())
    }

    /// `text` on at most `max_lines` lines of `max_width`, broken between words, or inside a word too long for a
    /// line. The last line ends with an ellipsis when the text goes on.
    pub fn wrap(&self, text: &str, px: f32, max_width: f32, max_lines: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let mut rest = text.trim();
        while !rest.is_empty() {
            if lines.len() + 1 == max_lines {
                lines.push(self.fit(rest, px, max_width));
                break;
            }
            let end = self.line_end(rest, px, max_width);
            lines.push(rest[..end].trim_end().to_string());
            rest = rest[end..].trim_start();
        }
        lines
    }

    /// Draws `text` from `x`, on the baseline `y`.
    pub fn draw(&mut self, image: &mut Image, text: &str, x: f32, y: f32, style: TextStyle) {
        let starts: Vec<(char, f32)> = self
            .layout(text, style.px)
            .map(|placed| (placed.ch, placed.start))
            .collect();
        for (ch, start) in starts {
            let (metrics, coverage) = self.glyph(ch, style.px);
            let left = (x + start + metrics.xmin as f32).round() as i32;
            let top = (y - metrics.ymin as f32 - metrics.height as f32).round() as i32;
            image.blend_mask(left, top, metrics.width, coverage, style);
        }
    }

    fn layout<'a>(&'a self, text: &'a str, px: f32) -> impl Iterator<Item = Placed> + 'a {
        let mut end = 0.0;
        let mut previous = None;
        text.char_indices().map(move |(index, ch)| {
            let kerning = previous.and_then(|previous| self.font.horizontal_kern(previous, ch, px));
            let start = end + kerning.unwrap_or(0.0);
            end = start + self.font.metrics(ch, px).advance_width;
            previous = Some(ch);
            Placed { index, ch, start, end }
        })
    }

    /// Where the first line of `text` ends: after the last word that fits, or where a longer word must break.
    fn line_end(&self, text: &str, px: f32, max_width: f32) -> usize {
        let mut fitting = 0;
        let mut last_space = None;
        for placed in self.layout(text, px) {
            if placed.end > max_width {
                break;
            }
            if placed.ch == ' ' {
                last_space = Some(placed.index);
            }
            fitting = placed.index + placed.ch.len_utf8();
        }
        if fitting == text.len() || text[fitting..].starts_with(' ') {
            return fitting;
        }
        let first_char = text.chars().next().map_or(0, char::len_utf8);
        last_space.unwrap_or(fitting).max(first_char)
    }

    fn glyph(&mut self, ch: char, px: f32) -> (Metrics, &[u8]) {
        let font = &self.font;
        let (metrics, coverage) = self
            .glyphs
            .entry((ch, px.to_bits()))
            .or_insert_with(|| font.rasterize(ch, px));
        (*metrics, coverage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_kerning() {
        let face = Face::medium();
        assert!(face.width("To", 12.0) < face.width("T", 12.0) + face.width("o", 12.0));
        let placed: Vec<Placed> = face.layout("To", 12.0).collect();
        assert!(placed[1].start < placed[0].end, "the o moves under the T");
    }

    #[test]
    fn drops_the_characters_it_cannot_draw() {
        let face = Face::medium();
        assert_eq!(face.printable("Airhorn 📯  \t è★"), "Airhorn è★");
    }

    #[test]
    fn wraps_between_words() {
        let face = Face::medium();
        let lines = face.wrap("Directed by Robert B. Weide", 12.0, 80.0, 2);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("Directed"), "{lines:?}");
        assert!(lines.iter().all(|line| face.width(line, 12.0) <= 80.0), "{lines:?}");
    }

    #[test]
    fn ends_with_an_ellipsis_when_the_name_goes_on() {
        let face = Face::medium();
        let lines = face.wrap("The quick brown fox jumps over the lazy dog", 12.0, 80.0, 2);
        assert_eq!(lines.len(), 2);
        assert!(lines[1].ends_with('…'), "{lines:?}");
        assert!(face.width(&lines[1], 12.0) <= 80.0);
    }

    #[test]
    fn breaks_a_word_longer_than_a_line() {
        let face = Face::medium();
        let lines = face.wrap("Supercalifragilisticexpialidocious", 12.0, 80.0, 2);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].len() > 3 && !lines[0].contains('…'), "{lines:?}");
        assert!(lines[1].ends_with('…'), "{lines:?}");
    }

    #[test]
    fn a_short_name_stays_on_one_line() {
        assert_eq!(Face::medium().wrap("Honk", 12.0, 80.0, 2), ["Honk"]);
    }
}
