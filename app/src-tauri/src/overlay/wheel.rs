//! The action wheel's picture: a dark disc of slices around Drezimo's logo, the one turned to lit in amber, as the
//! approved mockup draws it.

use super::draw::{BACKGROUND, BACKGROUND_ALPHA, Image, LINE, MUTED, Rect, TEXT};
use super::text::{Face, TextStyle};
use std::collections::HashMap;
use std::f32::consts::{PI, TAU};

// Sizes in points, multiplied by the screen's scale
const DISC: f32 = 190.0;
const OUTER: f32 = 180.0;
const INNER: f32 = 80.0;
const CENTRE: f32 = 70.0;
/// Between two slices
const GAP: f32 = 3.0;
/// Below the disc, for the hint
const HINT: f32 = 40.0;
const ICON: f32 = 24.0;
const NAME_SIZE: f32 = 13.5;
const NAME_LINE: f32 = 16.0;
const CENTRE_SIZE: f32 = 13.0;
const HINT_SIZE: f32 = 12.5;
/// The logo's 6×6 grid, from the edge of one square to the next
const LOGO_STEP: f32 = 7.5;

const AMBER: [u8; 3] = [255, 176, 32];
const SLICE: [u8; 3] = [27, 27, 32];
const SLICE_LINE: [u8; 3] = [44, 44, 51];
const CENTRE_FILL: [u8; 3] = [17, 17, 20];
const NAME: [u8; 3] = [201, 201, 207];
const SELECTED_FILL: f32 = 0.17;

#[derive(Clone, Debug, PartialEq)]
pub struct Wheel {
    /// Clockwise from the top
    pub slices: Vec<Slice>,
    /// The slice turned to, or None for the centre, where letting go does nothing
    pub selected: Option<usize>,
    /// Under the logo: the slice turned to, or what the centre does
    pub centre: String,
    /// Under the disc: how to pick
    pub hint: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Slice {
    pub name: String,
    pub icon: SliceIcon,
    /// The icon's, as the LED of a pad that does the same
    pub colour: [u8; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliceIcon {
    Page,
    Macro,
    Menu,
    StopSounds,
    MuteMic,
    MuteForMe,
    VoiceChanger,
    HearMyself,
    Background,
    RandomVoice,
    Overlay,
    ResetVolumes,
}

/// The wheel's size in pixels, on a screen of `scale`.
pub fn size(scale: f32) -> (usize, usize) {
    let side = 2.0 * DISC + 2.0;
    (
        (side * scale).round() as usize,
        ((side + HINT) * scale).round() as usize,
    )
}

pub struct WheelPainter {
    medium: Face,
    semibold: Face,
    /// Each name's lines, by name, room, scale and weight
    lines: HashMap<(String, u32, u32, bool), Vec<String>>,
}

impl WheelPainter {
    pub fn new() -> Self {
        Self {
            medium: Face::medium(),
            semibold: Face::semibold(),
            lines: HashMap::new(),
        }
    }

    pub fn draw(&mut self, wheel: &Wheel, scale: f32) -> Image {
        let (width, height) = size(scale);
        let mut image = Image::new(width, height);
        let centre = (DISC + 1.0) * scale;
        self.draw_disc(&mut image, wheel, centre, scale);
        draw_logo(&mut image, centre, centre - 14.0 * scale, scale);
        if self.lines.len() > 200 {
            self.lines.clear();
        }
        for (index, slice) in wheel.slices.iter().enumerate() {
            self.draw_slice(&mut image, wheel, index, slice, centre, scale);
        }
        let (face, color) = match wheel.selected {
            Some(_) => (&mut self.semibold, AMBER),
            None => (&mut self.medium, MUTED),
        };
        let px = CENTRE_SIZE * scale;
        let label = face.fit(&face.printable(&wheel.centre), px, 1.6 * CENTRE * scale);
        let x = centre - face.width(&label, px) / 2.0;
        face.draw(&mut image, &label, x, centre + 34.0 * scale, style(px, color, 1.0));
        self.draw_hint(&mut image, &wheel.hint, scale);
        image
    }

    /// The disc, the slices and the centre, each pixel by its distance to their edges
    fn draw_disc(&self, image: &mut Image, wheel: &Wheel, centre: f32, scale: f32) {
        let count = wheel.slices.len().max(1);
        let span = TAU / count as f32;
        let line = scale.round().max(1.0);
        let thick = 2.5 * scale;
        let side = ((2.0 * DISC + 2.0) * scale).ceil() as usize;
        for y in 0..side.min(image.height) {
            for x in 0..side.min(image.width) {
                let dx = x as f32 + 0.5 - centre;
                let dy = y as f32 + 0.5 - centre;
                let r = dx.hypot(dy);
                let disc = r - DISC * scale;
                if disc > 1.0 {
                    continue;
                }
                image.blend(x, y, BACKGROUND, coverage(disc) * BACKGROUND_ALPHA);
                image.blend(x, y, LINE, ring(disc, line));

                // Clockwise from the top
                let angle = dx.atan2(-dy).rem_euclid(TAU);
                let index = ((angle + span / 2.0) / span).floor() as usize % count;
                let local = (angle - index as f32 * span + PI).rem_euclid(TAU) - PI;
                let mut edge = (r - OUTER * scale).max(INNER * scale - r);
                if count > 1 {
                    let to_side = r * (span / 2.0 - local.abs()).clamp(0.0, PI / 2.0).sin();
                    edge = edge.max(GAP * scale / 2.0 - to_side);
                }
                let selected = wheel.selected == Some(index);
                if selected {
                    image.blend(x, y, AMBER, coverage(edge) * SELECTED_FILL);
                    image.blend(x, y, AMBER, ring(edge, thick));
                } else {
                    image.blend(x, y, SLICE, coverage(edge));
                    image.blend(x, y, SLICE_LINE, ring(edge, line));
                }

                let inside = r - CENTRE * scale;
                image.blend(x, y, CENTRE_FILL, coverage(inside));
                if wheel.selected.is_none() {
                    image.blend(x, y, AMBER, ring(inside, thick));
                } else {
                    image.blend(x, y, SLICE_LINE, ring(inside, line));
                }
            }
        }
    }

    fn draw_slice(&mut self, image: &mut Image, wheel: &Wheel, index: usize, slice: &Slice, centre: f32, scale: f32) {
        let count = wheel.slices.len().max(1);
        let span = TAU / count as f32;
        let angle = index as f32 * span;
        let middle = (OUTER + INNER) / 2.0 + 2.0;
        let x = centre + middle * scale * angle.sin();
        let y = centre - middle * scale * angle.cos();
        let room = if count > 1 {
            (2.0 * middle * (span / 2.0).sin() - 18.0).min(116.0)
        } else {
            140.0
        };
        let selected = wheel.selected == Some(index);
        let px = NAME_SIZE * scale;
        let face = if selected { &self.semibold } else { &self.medium };
        let lines = self
            .lines
            .entry((slice.name.clone(), room.to_bits(), px.to_bits(), selected))
            .or_insert_with(|| face.wrap(&face.printable(&slice.name), px, room * scale, 2))
            .clone();
        let height = (ICON + 6.0 + NAME_LINE * lines.len() as f32) * scale;
        let top = y - height / 2.0;
        draw_icon(image, slice.icon, x - ICON * scale / 2.0, top, scale, slice.colour);
        let face = if selected { &mut self.semibold } else { &mut self.medium };
        let baseline = face.baseline(px, NAME_LINE * scale);
        let color = if selected { TEXT } else { NAME };
        for (number, line) in lines.iter().enumerate() {
            let line_top = top + (ICON + 6.0 + NAME_LINE * number as f32) * scale;
            let left = x - face.width(line, px) / 2.0;
            face.draw(image, line, left, line_top + baseline, style(px, color, 1.0));
        }
    }

    /// How to pick, on a dark pill under the disc, readable over any game
    fn draw_hint(&mut self, image: &mut Image, hint: &str, scale: f32) {
        if hint.is_empty() {
            return;
        }
        let px = HINT_SIZE * scale;
        let face = &mut self.medium;
        let text = face.fit(&face.printable(hint), px, (2.0 * DISC - 24.0) * scale);
        let text_width = face.width(&text, px);
        let pill_height = 26.0 * scale;
        let pill = Rect {
            x: ((image.width as f32 - text_width) / 2.0 - 12.0 * scale).round(),
            y: ((2.0 * DISC + 2.0 + 8.0) * scale).round(),
            width: (text_width + 24.0 * scale).round(),
            height: pill_height.round(),
        };
        image.fill_rounded_rect(pill, pill.height / 2.0, BACKGROUND, BACKGROUND_ALPHA);
        image.stroke_rounded_rect(pill, pill.height / 2.0, scale.round().max(1.0), LINE, 1.0);
        let baseline = pill.y + face.baseline(px, pill.height);
        face.draw(image, &text, pill.x + 12.0 * scale, baseline, style(px, TEXT, 0.8));
    }
}

fn style(px: f32, color: [u8; 3], alpha: f32) -> TextStyle {
    TextStyle { px, color, alpha }
}

fn coverage(distance: f32) -> f32 {
    (0.5 - distance).clamp(0.0, 1.0)
}

/// A border `width` pixels wide inside a shape, from the distance to its edge
fn ring(distance: f32, width: f32) -> f32 {
    coverage(distance) - coverage(distance + width)
}

/// Drezimo's logo as on its icon: amber squares drawing a D on a 6×6 grid, the other places small grey dots
fn draw_logo(image: &mut Image, x: f32, y: f32, scale: f32) {
    let step = LOGO_STEP * scale;
    let left = x - 3.0 * step;
    let top = y - 3.0 * step;
    for row in 0..6 {
        for column in 0..6 {
            let edge_row = row == 0 || row == 5;
            let lit = if edge_row {
                column < 5
            } else {
                column == 0 || column == 5
            };
            let (size, inset, color, alpha) = if lit {
                (0.8 * step, 0.1 * step, AMBER, 1.0)
            } else {
                (0.4 * step, 0.3 * step, TEXT, 0.4)
            };
            let rect = Rect {
                x: left + column as f32 * step + inset,
                y: top + row as f32 * step + inset,
                width: size,
                height: size,
            };
            image.fill_rounded_rect(rect, size * 0.22, color, alpha);
        }
    }
}

/// An icon drawn as the window's: lines 2 units wide on a 24-unit square, `ICON` points big, its top-left at (x, y)
fn draw_icon(image: &mut Image, icon: SliceIcon, x: f32, y: f32, scale: f32, color: [u8; 3]) {
    let unit = ICON * scale / 24.0;
    let lines = icon_lines(icon);
    let half = unit;
    let left = x.floor().max(0.0) as usize;
    let top = y.floor().max(0.0) as usize;
    let right = ((x + ICON * scale).ceil() as usize + 1).min(image.width);
    let bottom = ((y + ICON * scale).ceil() as usize + 1).min(image.height);
    for py in top..bottom {
        for px in left..right {
            let point = ((px as f32 + 0.5 - x) / unit, (py as f32 + 0.5 - y) / unit);
            let nearest = lines
                .iter()
                .flat_map(|line| line.windows(2))
                .map(|pair| to_segment(point, pair[0], pair[1]))
                .fold(f32::MAX, f32::min);
            let alpha = coverage(nearest * unit - half);
            if alpha > 0.0 {
                image.blend(px, py, color, alpha);
            }
        }
    }
}

fn to_segment(point: (f32, f32), from: (f32, f32), to: (f32, f32)) -> f32 {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let length = dx * dx + dy * dy;
    let along = if length == 0.0 {
        0.0
    } else {
        (((point.0 - from.0) * dx + (point.1 - from.1) * dy) / length).clamp(0.0, 1.0)
    };
    (point.0 - from.0 - along * dx).hypot(point.1 - from.1 - along * dy)
}

/// An arc around (x, y), from one angle to another in degrees, clockwise on screen as they grow
fn arc(x: f32, y: f32, radius: f32, from: f32, to: f32) -> Vec<(f32, f32)> {
    let steps = ((to - from).abs() / 10.0).ceil().max(2.0) as usize;
    (0..=steps)
        .map(|step| {
            let angle = (from + (to - from) * step as f32 / steps as f32).to_radians();
            (x + radius * angle.cos(), y + radius * angle.sin())
        })
        .collect()
}

fn closed(points: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut points = points.to_vec();
    points.push(points[0]);
    points
}

/// A rectangle with its corners rounded by `radius`
fn rounded(left: f32, top: f32, right: f32, bottom: f32, radius: f32) -> Vec<(f32, f32)> {
    let mut points = arc(right - radius, top + radius, radius, 270.0, 360.0);
    points.extend(arc(right - radius, bottom - radius, radius, 0.0, 90.0));
    points.extend(arc(left + radius, bottom - radius, radius, 90.0, 180.0));
    points.extend(arc(left + radius, top + radius, radius, 180.0, 270.0));
    closed(&points)
}

/// A cubic Bézier curve, as SVG draws it
fn cubic(from: (f32, f32), first: (f32, f32), second: (f32, f32), to: (f32, f32)) -> Vec<(f32, f32)> {
    (0..=12)
        .map(|step| {
            let t = step as f32 / 12.0;
            let u = 1.0 - t;
            let weights = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
            let points = [from, first, second, to];
            let x = weights.iter().zip(points).map(|(weight, point)| weight * point.0).sum();
            let y = weights.iter().zip(points).map(|(weight, point)| weight * point.1).sum();
            (x, y)
        })
        .collect()
}

/// Two waves, as the window draws Voicemod's background effects
fn wave(y: f32) -> Vec<(f32, f32)> {
    let mut points = cubic((3.0, y), (6.0, y), (6.0, y - 3.0), (9.0, y - 3.0));
    points.extend(cubic((9.0, y - 3.0), (12.0, y - 3.0), (12.0, y), (15.0, y)));
    points.extend(cubic((15.0, y), (18.0, y), (18.0, y - 3.0), (21.0, y - 3.0)));
    points
}

/// The window's icons (src/icons.tsx), as lines
fn icon_lines(icon: SliceIcon) -> Vec<Vec<(f32, f32)>> {
    match icon {
        SliceIcon::Page => {
            let mut back = vec![(8.0, 3.0)];
            back.extend(arc(19.0, 5.0, 2.0, 270.0, 360.0));
            back.push((21.0, 16.0));
            vec![back, rounded(3.0, 7.0, 17.0, 21.0, 2.0)]
        }
        SliceIcon::Macro => vec![
            vec![(4.0, 6.0), (13.0, 6.0)],
            vec![(4.0, 12.0), (10.0, 12.0)],
            vec![(4.0, 18.0), (13.0, 18.0)],
            closed(&[(16.0, 9.0), (20.0, 12.0), (16.0, 15.0)]),
        ],
        SliceIcon::Menu => [3.0, 9.5, 16.0]
            .into_iter()
            .flat_map(|y| [3.0, 9.5, 16.0].map(|x| rounded(x, y, x + 5.0, y + 5.0, 1.0)))
            .collect(),
        SliceIcon::StopSounds => vec![rounded(6.0, 6.0, 18.0, 18.0, 2.0)],
        SliceIcon::MuteMic => {
            let mut top = vec![(15.0, 10.5)];
            top.extend(arc(12.0, 6.0, 3.0, 0.0, -160.0));
            let mut bottom = vec![(9.0, 9.0)];
            bottom.extend(arc(12.0, 11.0, 3.0, 180.0, 47.7));
            vec![
                top,
                bottom,
                arc(12.0, 11.0, 6.5, 180.0, 48.7),
                arc(12.0, 11.0, 6.5, 0.0, 22.0),
                vec![(12.0, 17.5), (12.0, 21.0)],
                vec![(4.0, 4.0), (20.0, 20.0)],
            ]
        }
        SliceIcon::MuteForMe => vec![
            closed(&[
                (11.0, 5.0),
                (6.5, 9.0),
                (3.5, 9.0),
                (3.5, 15.0),
                (6.5, 15.0),
                (11.0, 19.0),
            ]),
            vec![(16.0, 9.5), (21.0, 14.5)],
            vec![(21.0, 9.5), (16.0, 14.5)],
        ],
        SliceIcon::VoiceChanger => [(4.0, 10.0), (8.0, 7.0), (12.0, 4.0), (16.0, 8.0), (20.0, 10.5)]
            .into_iter()
            .map(|(x, top)| vec![(x, top), (x, 24.0 - top)])
            .collect(),
        SliceIcon::HearMyself => {
            let mut band = vec![(4.0, 15.0)];
            band.extend(arc(12.0, 12.0, 8.0, 180.0, 360.0));
            band.push((20.0, 15.0));
            vec![
                band,
                rounded(3.0, 14.0, 7.5, 20.5, 1.5),
                rounded(16.5, 14.0, 21.0, 20.5, 1.5),
            ]
        }
        SliceIcon::Background => vec![wave(16.0), wave(10.0)],
        SliceIcon::RandomVoice => {
            let mut across = vec![(4.0, 7.0)];
            across.extend(cubic((7.0, 7.0), (11.5, 7.0), (12.5, 17.0), (17.0, 17.0)));
            across.push((20.0, 17.0));
            let mut low = vec![(4.0, 17.0)];
            low.extend(cubic((7.0, 17.0), (8.7, 17.0), (9.9, 15.6), (10.9, 13.7)));
            let mut high = cubic((13.1, 10.3), (14.1, 8.4), (15.3, 7.0), (17.0, 7.0));
            high.push((20.0, 7.0));
            vec![
                across,
                low,
                high,
                vec![(17.5, 4.5), (20.0, 7.0), (17.5, 9.5)],
                vec![(17.5, 14.5), (20.0, 17.0), (17.5, 19.5)],
            ]
        }
        SliceIcon::Overlay => vec![
            rounded(2.0, 4.0, 22.0, 18.0, 2.0),
            rounded(12.0, 10.0, 19.0, 15.0, 1.0),
            vec![(8.0, 21.0), (16.0, 21.0)],
        ],
        SliceIcon::ResetVolumes => vec![
            arc(12.0, 12.0, 8.0, 180.0, -135.0),
            vec![(4.0, 4.0), (4.0, 8.0), (8.0, 8.0)],
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(selected: Option<usize>) -> Wheel {
        let slice = |name: &str, icon| Slice {
            name: name.into(),
            icon,
            colour: [255, 42, 42],
        };
        Wheel {
            slices: vec![
                slice("Meme", SliceIcon::Page),
                slice("Stop sounds", SliceIcon::StopSounds),
                slice("Mute mic", SliceIcon::MuteMic),
                slice("Intro live", SliceIcon::Macro),
            ],
            selected,
            centre: "Cancel".into(),
            hint: "Let go to pick".into(),
        }
    }

    /// Halfway along the first slice, at the top
    fn top_slice(scale: f32) -> (usize, usize) {
        let centre = (DISC + 1.0) * scale;
        (centre as usize, (centre - (OUTER - 6.0) * scale) as usize)
    }

    #[test]
    fn is_a_disc_with_the_hint_below() {
        assert_eq!(size(1.0), (382, 422));
        let image = WheelPainter::new().draw(&sample(None), 1.0);
        assert_eq!(image.pixel(0, 0)[3], 0, "outside the disc");
        assert!(image.pixel(191, 191)[3] > 200, "the centre");
    }

    #[test]
    fn lights_the_slice_turned_to_in_amber() {
        let (x, y) = top_slice(1.0);
        let plain = WheelPainter::new().draw(&sample(None), 1.0).pixel(x, y);
        let lit = WheelPainter::new().draw(&sample(Some(0)), 1.0).pixel(x, y);
        assert!(lit[0] > plain[0] + 20, "{lit:?} against {plain:?}");
        assert!(lit[0] > lit[2], "amber");
    }

    #[test]
    fn leaves_a_gap_between_slices() {
        let image = WheelPainter::new().draw(&sample(None), 1.0);
        // On the line between the first slice and the second, at 45°
        let centre = DISC + 1.0;
        let along = (OUTER + INNER) / 2.0 / 2f32.sqrt();
        let gap = image.pixel((centre + along) as usize, (centre - along) as usize);
        let inside = image.pixel(centre as usize, (centre - (OUTER + INNER) / 2.0) as usize + 40);
        assert!(gap[0] < 20 && inside[0] >= 20, "{gap:?} {inside:?}");
    }

    #[test]
    fn draws_at_the_screen_s_scale() {
        let image = WheelPainter::new().draw(&sample(Some(1)), 2.0);
        assert_eq!((image.width, image.height), size(2.0));
    }
}
