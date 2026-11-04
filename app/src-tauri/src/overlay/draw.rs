//! The overlay's picture: a dark panel with the page's name and the 6×6 pads, each with its sound's name and its LED's
//! colour, as the simulator previews it.

use super::text::{Face, TextStyle};
use super::{Cell, Overlay};
use crate::protocol::Effect;
use std::collections::HashMap;

const COLUMNS: usize = 6;

// Sizes in points, multiplied by the screen's scale
const PADDING: f32 = 12.0;
const HEADER: f32 = 22.0;
const CELL_WIDTH: f32 = 92.0;
const CELL_HEIGHT: f32 = 38.0;
const GAP: f32 = 4.0;
pub(super) const PANEL_RADIUS: f32 = 12.0;
const CELL_RADIUS: f32 = 6.0;
const CELL_PADDING: f32 = 6.0;
const TITLE_SIZE: f32 = 13.0;
const COUNTER_SIZE: f32 = 12.0;
const NAME_SIZE: f32 = 12.0;
const NAME_LINE: f32 = 14.0;
const NAME_LINES: usize = 2;

// The settings window's colours (src/styles.css)
pub(super) const BACKGROUND: [u8; 3] = [14, 14, 16];
pub(super) const BACKGROUND_ALPHA: f32 = 0.94;
pub(super) const LINE: [u8; 3] = [38, 38, 44];
pub(super) const TEXT: [u8; 3] = [244, 243, 239];
pub(super) const MUTED: [u8; 3] = [139, 139, 148];
/// How much an LED's colour tints its pad, as in the simulator
const TINT: f32 = 0.18;
/// A sound outside the licence, as in the simulator
const LOCKED_ALPHA: f32 = 0.45;

/// A picture with its transparency: RGBA, each colour already multiplied by its alpha.
#[derive(Clone)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

#[derive(Clone, Copy)]
pub(super) struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// The overlay's size in pixels, on a screen of `scale`.
pub fn size(scale: f32) -> (usize, usize) {
    let width = 2.0 * PADDING + COLUMNS as f32 * CELL_WIDTH + (COLUMNS - 1) as f32 * GAP;
    let height = 2.0 * PADDING + HEADER + COLUMNS as f32 * CELL_HEIGHT + (COLUMNS - 1) as f32 * GAP;
    ((width * scale).round() as usize, (height * scale).round() as usize)
}

pub struct Painter {
    medium: Face,
    semibold: Face,
    /// Each name's lines, by name and scale: names change far less often than the LEDs
    lines: HashMap<(String, u32), Vec<String>>,
    /// The picture without its animated pads, and the overlay and scale it shows
    still: Option<(Overlay, u32, Image)>,
}

impl Painter {
    pub fn new() -> Self {
        Self {
            medium: Face::medium(),
            semibold: Face::semibold(),
            lines: HashMap::new(),
            still: None,
        }
    }

    /// The overlay on a screen of `scale`, each animated LED at its effect's `level` (0 to 1). Only the animated pads
    /// are drawn again while the overlay stays the same.
    pub fn draw(&mut self, overlay: &Overlay, scale: f32, level: impl Fn(&Cell) -> f32) -> Image {
        let current = self
            .still
            .as_ref()
            .is_some_and(|(shown, shown_scale, _)| shown == overlay && *shown_scale == scale.to_bits());
        if !current {
            let still = self.draw_still(overlay, scale);
            self.still = Some((overlay.clone(), scale.to_bits(), still));
        }
        let mut image = self
            .still
            .as_ref()
            .map(|(_, _, still)| still.clone())
            .unwrap_or_else(|| Image::new(0, 0));
        for (index, cell) in overlay.cells.iter().enumerate() {
            if cell.led.effect != Effect::Solid {
                self.draw_cell(&mut image, cell, index, scale, level(cell));
            }
        }
        image
    }

    /// The panel, the header and the pads whose LEDs do not change by themselves.
    fn draw_still(&mut self, overlay: &Overlay, scale: f32) -> Image {
        let (width, height) = size(scale);
        let mut image = Image::new(width, height);
        let panel = Rect {
            x: 0.0,
            y: 0.0,
            width: width as f32,
            height: height as f32,
        };
        image.fill_rounded_rect(panel, PANEL_RADIUS * scale, BACKGROUND, BACKGROUND_ALPHA);
        image.stroke_rounded_rect(panel, PANEL_RADIUS * scale, scale.round(), LINE, 1.0);

        self.draw_header(&mut image, overlay, scale);
        if self.lines.len() > 1000 {
            self.lines.clear();
        }
        for (index, cell) in overlay.cells.iter().enumerate() {
            if cell.led.effect == Effect::Solid {
                self.draw_cell(&mut image, cell, index, scale, 1.0);
            }
        }
        image
    }

    fn draw_header(&mut self, image: &mut Image, overlay: &Overlay, scale: f32) {
        let counter_size = COUNTER_SIZE * scale;
        let counter_width = self.medium.width(&overlay.counter, counter_size);
        let right = image.width as f32 - PADDING * scale;
        let baseline = PADDING * scale + self.semibold.baseline(TITLE_SIZE * scale, HEADER * scale - GAP * scale);
        let counter_x = right - counter_width;
        let counter_style = TextStyle {
            px: counter_size,
            color: MUTED,
            alpha: 1.0,
        };
        self.medium
            .draw(image, &overlay.counter, counter_x, baseline, counter_style);

        let title_size = TITLE_SIZE * scale;
        let room = counter_x - PADDING * scale - 2.0 * PADDING * scale;
        let title = self
            .semibold
            .fit(&self.semibold.printable(&overlay.title), title_size, room);
        let title_style = TextStyle {
            px: title_size,
            color: TEXT,
            alpha: 1.0,
        };
        self.semibold
            .draw(image, &title, PADDING * scale, baseline, title_style);
    }

    fn draw_cell(&mut self, image: &mut Image, cell: &Cell, index: usize, scale: f32, level: f32) {
        let (row, column) = (index / COLUMNS, index % COLUMNS);
        // Whole pixels, so the borders stay sharp
        let x = (PADDING + column as f32 * (CELL_WIDTH + GAP)) * scale;
        let y = (PADDING + HEADER + row as f32 * (CELL_HEIGHT + GAP)) * scale;
        let rect = Rect {
            x: x.round(),
            y: y.round(),
            width: (CELL_WIDTH * scale).round(),
            height: (CELL_HEIGHT * scale).round(),
        };
        let alpha = if cell.locked { LOCKED_ALPHA } else { 1.0 };
        let color = cell.led.color.map(|channel| (f32::from(channel) * level).round() as u8);
        let lit = color.iter().any(|channel| *channel > 0);
        let radius = CELL_RADIUS * scale;
        if lit {
            image.fill_rounded_rect(rect, radius, color, TINT * alpha);
        }
        let border = if lit { color } else { LINE };
        image.stroke_rounded_rect(rect, radius, scale.round(), border, alpha);

        let size = NAME_SIZE * scale;
        let room = rect.width - 2.0 * CELL_PADDING * scale;
        let medium = &self.medium;
        let lines = self
            .lines
            .entry((cell.name.clone(), scale.to_bits()))
            .or_insert_with(|| medium.wrap(&medium.printable(&cell.name), size, room, NAME_LINES))
            .clone();
        let line_height = NAME_LINE * scale;
        let top = rect.y + (rect.height - line_height * lines.len() as f32) / 2.0;
        let baseline = self.medium.baseline(size, line_height);
        let style = TextStyle {
            px: size,
            color: TEXT,
            alpha,
        };
        for (number, line) in lines.iter().enumerate() {
            let y = top + number as f32 * line_height + baseline;
            self.medium.draw(image, line, rect.x + CELL_PADDING * scale, y, style);
        }
    }
}

impl Image {
    pub(super) fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width * height * 4],
        }
    }

    /// The pixel at (x, y), as RGBA with the colours multiplied by the alpha.
    #[cfg(test)]
    pub fn pixel(&self, x: usize, y: usize) -> [u8; 4] {
        let index = (y * self.width + x) * 4;
        self.pixels[index..index + 4].try_into().unwrap()
    }

    /// Paints `color` over the pixel at (x, y), with `alpha` from 0 (nothing) to 1 (covers it).
    fn blend(&mut self, x: usize, y: usize, color: [u8; 3], alpha: f32) {
        let index = (y * self.width + x) * 4;
        let pixel = &mut self.pixels[index..index + 4];
        let behind = 1.0 - alpha;
        for (channel, value) in pixel.iter_mut().zip(color.iter().chain([&255])) {
            *channel = (f32::from(*value) * alpha + f32::from(*channel) * behind).round() as u8;
        }
    }

    /// Paints a glyph's coverage in the style's colour, its top-left corner at (left, top).
    pub(super) fn blend_mask(&mut self, left: i32, top: i32, width: usize, coverage: &[u8], style: TextStyle) {
        for (index, value) in coverage.iter().enumerate() {
            let x = left + (index % width.max(1)) as i32;
            let y = top + (index / width.max(1)) as i32;
            if *value > 0 && (0..self.width as i32).contains(&x) && (0..self.height as i32).contains(&y) {
                self.blend(
                    x as usize,
                    y as usize,
                    style.color,
                    style.alpha * f32::from(*value) / 255.0,
                );
            }
        }
    }

    pub(super) fn fill_rounded_rect(&mut self, rect: Rect, radius: f32, color: [u8; 3], alpha: f32) {
        self.paint_rounded_rect(rect, radius, |distance| coverage(distance) * alpha, color);
    }

    /// A border `width` pixels wide, inside the rectangle.
    pub(super) fn stroke_rounded_rect(&mut self, rect: Rect, radius: f32, width: f32, color: [u8; 3], alpha: f32) {
        let ring = |distance: f32| (coverage(distance) - coverage(distance + width)) * alpha;
        self.paint_rounded_rect(rect, radius, ring, color);
    }

    /// Covers whole pixels with `color`, opaque.
    pub(super) fn fill_pixels(&mut self, x: usize, y: usize, width: usize, height: usize, color: [u8; 3]) {
        for row in y..(y + height).min(self.height) {
            for column in x..(x + width).min(self.width) {
                let index = (row * self.width + column) * 4;
                self.pixels[index..index + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
            }
        }
    }

    /// Paints each pixel of the rectangle with the alpha its distance from the rounded edge gives (negative inside).
    fn paint_rounded_rect(&mut self, rect: Rect, radius: f32, alpha_at: impl Fn(f32) -> f32, color: [u8; 3]) {
        let half_width = rect.width / 2.0;
        let half_height = rect.height / 2.0;
        let center_x = rect.x + half_width;
        let center_y = rect.y + half_height;
        let radius = radius.min(half_width).min(half_height);
        let left = rect.x.floor().max(0.0) as usize;
        let top = rect.y.floor().max(0.0) as usize;
        let right = ((rect.x + rect.width).ceil() as usize).min(self.width);
        let bottom = ((rect.y + rect.height).ceil() as usize).min(self.height);
        for y in top..bottom {
            for x in left..right {
                // The distance from the pixel's centre to the rounded rectangle
                let dx = (x as f32 + 0.5 - center_x).abs() - (half_width - radius);
                let dy = (y as f32 + 0.5 - center_y).abs() - (half_height - radius);
                let outside = dx.max(0.0).hypot(dy.max(0.0));
                let distance = outside + dx.max(dy).min(0.0) - radius;
                let alpha = alpha_at(distance);
                if alpha > 0.0 {
                    self.blend(x, y, color, alpha.min(1.0));
                }
            }
        }
    }
}

/// How much of a pixel a shape covers, from the distance of its centre to the shape's edge.
fn coverage(distance: f32) -> f32 {
    (0.5 - distance).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Effect, Led};

    fn sample() -> Overlay {
        let mut cells = vec![
            Cell {
                name: String::new(),
                led: Led::OFF,
                locked: false,
            };
            36
        ];
        cells[0] = Cell {
            name: "Airhorn".into(),
            led: Led::solid([34, 224, 90]),
            locked: false,
        };
        cells[1] = Cell {
            name: "Blinking".into(),
            led: Led {
                color: [200, 200, 210],
                effect: Effect::Blink,
            },
            locked: false,
        };
        Overlay {
            title: "★ Favourites".into(),
            counter: "1/3".into(),
            cells,
        }
    }

    /// The first pad's left border, halfway down, at scale 1
    const FIRST_BORDER: (usize, usize) = (12, 34 + 19);

    #[test]
    fn has_the_simulator_s_proportions() {
        assert_eq!(size(1.0), (596, 294));
        assert_eq!(size(2.0), (1192, 588));
    }

    #[test]
    fn draws_a_dark_panel_with_round_corners() {
        let image = Painter::new().draw(&sample(), 1.0, |_| 1.0);
        assert_eq!((image.width, image.height), size(1.0));
        assert_eq!(image.pixel(0, 0)[3], 0, "outside the corner");
        let inside = image.pixel(5, 150);
        assert_eq!(inside[3], 240);
        assert_eq!(inside[0], (14.0f32 * 0.94).round() as u8);
    }

    #[test]
    fn lit_pads_show_their_led_colour() {
        let image = Painter::new().draw(&sample(), 1.0, |_| 1.0);
        let (x, y) = FIRST_BORDER;
        assert_eq!(image.pixel(x, y), [34, 224, 90, 255]);
        assert!(
            image.pixel(x + 1, y)[1] > image.pixel(x + 1, y)[0],
            "tinted green inside"
        );
    }

    #[test]
    fn a_blinking_pad_goes_dark() {
        let image = Painter::new().draw(&sample(), 1.0, |cell| match cell.led.effect {
            Effect::Blink => 0.0,
            _ => 1.0,
        });
        let (x, y) = FIRST_BORDER;
        let second = x + (CELL_WIDTH + GAP) as usize;
        assert_eq!(image.pixel(second, y), [38, 38, 44, 255]);
    }

    #[test]
    fn redraws_only_the_animated_pads_between_frames() {
        let mut painter = Painter::new();
        let blinking = |on: f32| move |cell: &Cell| if cell.led.effect == Effect::Blink { on } else { 1.0 };
        let (x, y) = FIRST_BORDER;
        let second = x + (CELL_WIDTH + GAP) as usize;
        assert_eq!(
            painter.draw(&sample(), 1.0, blinking(1.0)).pixel(second, y),
            [200, 200, 210, 255]
        );
        let dark = painter.draw(&sample(), 1.0, blinking(0.0));
        assert_eq!(dark.pixel(second, y), [38, 38, 44, 255]);
        assert_eq!(dark.pixel(x, y), [34, 224, 90, 255]);
    }

    #[test]
    fn writes_the_names() {
        let image = Painter::new().draw(&sample(), 1.0, |_| 1.0);
        let (x, y) = FIRST_BORDER;
        let bright = (x + 6..x + 60)
            .flat_map(|column| (y - 8..y + 8).map(move |row| (column, row)))
            .filter(|(column, row)| image.pixel(*column, *row)[0] > 200)
            .count();
        assert!(bright > 20, "{bright} bright pixels");
    }
}
