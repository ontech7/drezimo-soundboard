//! The board test, for a board just built: every key and the knob ticked as they are used, the LEDs one colour at a
//! time and test pictures on the display. Meanwhile the board plays nothing.

use crate::display::{Canvas, Frame, HEIGHT, Size, WIDTH};
use crate::protocol::{self, BoardMessage, LED_COUNT, Led, PAD_COUNT};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// The LEDs' brightness at most, in percent, as a board's first power-ups want (docs/project.md, section 12)
pub const TEST_BRIGHTNESS: u8 = 30;
/// The display dims after a minute without input at the shortest: it is woken up more often while it is looked at
const KEEP_AWAKE: Duration = Duration::from_secs(30);
const HELD: [u8; 3] = [255, 255, 255];
const TESTED: [u8; 3] = [0, 255, 0];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LedTest {
    /// Each key lit while held, then green once tested
    #[default]
    Keys,
    Red,
    Green,
    Blue,
    White,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DisplayTest {
    /// The edges of the area the screens use, with text in the middle
    #[default]
    Frame,
    /// Every pixel lit, to find the dead ones
    Full,
    Checkers,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardTest {
    pub leds: LedTest,
    pub display: DisplayTest,
    /// Each key pressed at least once, numbered as the LEDs: the pads, then the function keys
    pub tested: Vec<bool>,
    pub held: Vec<bool>,
    pub knob: Knob,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Knob {
    pub left: bool,
    pub right: bool,
    pub pressed: bool,
    pub held: bool,
}

impl Default for BoardTest {
    fn default() -> Self {
        Self {
            leds: LedTest::default(),
            display: DisplayTest::default(),
            tested: vec![false; LED_COUNT],
            held: vec![false; LED_COUNT],
            knob: Knob::default(),
        }
    }
}

/// A test running, and what the board shows of it
pub struct Tester {
    test: BoardTest,
    leds: [Option<Led>; LED_COUNT],
    display: Option<DisplayTest>,
    woken: Option<Instant>,
}

impl Default for Tester {
    fn default() -> Self {
        Self {
            test: BoardTest::default(),
            leds: [None; LED_COUNT],
            display: None,
            woken: None,
        }
    }
}

impl Tester {
    pub fn test(&self) -> &BoardTest {
        &self.test
    }

    pub fn board_message(&mut self, message: &BoardMessage) {
        match *message {
            BoardMessage::Pad { index, pressed } => self.key(index.into(), pressed),
            BoardMessage::Button { index, pressed } => self.key((PAD_COUNT + index).into(), pressed),
            BoardMessage::Encoder { step } if step < 0 => self.test.knob.left = true,
            BoardMessage::Encoder { .. } => self.test.knob.right = true,
            BoardMessage::EncoderButton { pressed } => {
                self.test.knob.held = pressed;
                self.test.knob.pressed |= pressed;
            }
            _ => {}
        }
    }

    pub fn set_leds(&mut self, leds: LedTest) {
        self.test.leds = leds;
    }

    /// Wakes the display too, to look at the new picture.
    pub fn set_display(&mut self, display: DisplayTest) {
        self.test.display = display;
        self.woken = None;
    }

    /// Clears the ticks, for another board
    pub fn restart(&mut self) {
        self.test = BoardTest {
            leds: self.test.leds,
            display: self.test.display,
            ..BoardTest::default()
        };
    }

    /// A board that connects again needs everything again, and cannot report the release of what was held.
    pub fn disconnected(&mut self) {
        self.leds = [None; LED_COUNT];
        self.display = None;
        self.woken = None;
        self.test.held.fill(false);
        self.test.knob.held = false;
    }

    /// The messages that bring the board to what the test shows, and keep its display awake.
    pub fn messages(&mut self, now: Instant) -> Vec<String> {
        let mut messages = Vec::new();
        if self.woken.is_none_or(|woken| now >= woken + KEEP_AWAKE) {
            self.woken = Some(now);
            messages.push("WAKE".to_string());
        }
        for index in 0..LED_COUNT {
            let led = self.led(index);
            if self.leds[index] != Some(led) {
                self.leds[index] = Some(led);
                messages.push(led.message(index));
            }
        }
        if self.display != Some(self.test.display) {
            self.display = Some(self.test.display);
            messages.push(protocol::display_message(&self.picture().hex()));
        }
        messages
    }

    /// When the display needs waking up again
    pub fn next_update(&self) -> Option<Instant> {
        self.woken.map(|woken| woken + KEEP_AWAKE)
    }

    pub fn picture(&self) -> Frame {
        canvas(self.test.display).frame()
    }

    fn key(&mut self, index: usize, pressed: bool) {
        self.test.held[index] = pressed;
        self.test.tested[index] |= pressed;
    }

    fn led(&self, index: usize) -> Led {
        Led::solid(match self.test.leds {
            LedTest::Keys if self.test.held[index] => HELD,
            LedTest::Keys if self.test.tested[index] => TESTED,
            LedTest::Keys => [0, 0, 0],
            LedTest::Red => [255, 0, 0],
            LedTest::Green => [0, 255, 0],
            LedTest::Blue => [0, 0, 255],
            LedTest::White => [255, 255, 255],
        })
    }
}

/// Inside the pixel free on each side that the screens keep: the pixel shift moves them by one and leaves the edge
/// it uncovers dark, so a picture reaching the edges would look cut, or show dark pixels that are fine.
fn canvas(test: DisplayTest) -> Canvas {
    let (left, top, right, bottom) = (1, 1, WIDTH - 2, HEIGHT - 2);
    let mut canvas = Canvas::default();
    match test {
        DisplayTest::Frame => {
            canvas.fill_rect(left, top, right - left + 1, 1, true);
            canvas.fill_rect(left, bottom, right - left + 1, 1, true);
            canvas.fill_rect(left, top, 1, bottom - top + 1, true);
            canvas.fill_rect(right, top, 1, bottom - top + 1, true);
            canvas.text_centered("DREZIMO", WIDTH / 2, 18, Size::Big, true);
            canvas.text_centered(&format!("{WIDTH} x {HEIGHT}"), WIDTH / 2, 40, Size::Small, true);
        }
        DisplayTest::Full => canvas.fill_rect(left, top, right - left + 1, bottom - top + 1, true),
        DisplayTest::Checkers => {
            for y in top..=bottom {
                for x in (left + (y + left) % 2..=right).step_by(2) {
                    canvas.fill_rect(x, y, 1, 1, true);
                }
            }
        }
    }
    canvas
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(messages: &[String]) -> Vec<&str> {
        messages
            .iter()
            .filter(|message| message.starts_with("LED,") && !message.ends_with(",0,0,0"))
            .map(String::as_str)
            .collect()
    }

    #[test]
    fn ticks_keys_and_the_knob_as_they_are_used() {
        let mut tester = Tester::default();
        tester.board_message(&BoardMessage::Pad {
            index: 3,
            pressed: true,
        });
        tester.board_message(&BoardMessage::Button {
            index: 5,
            pressed: true,
        });
        tester.board_message(&BoardMessage::Button {
            index: 5,
            pressed: false,
        });
        tester.board_message(&BoardMessage::Encoder { step: -1 });
        tester.board_message(&BoardMessage::EncoderButton { pressed: true });

        let test = tester.test();
        assert!(test.tested[3] && test.held[3]);
        assert!(test.tested[41] && !test.held[41]);
        assert_eq!(test.tested.iter().filter(|tested| **tested).count(), 2);
        assert_eq!(
            test.knob,
            Knob {
                left: true,
                right: false,
                pressed: true,
                held: true
            }
        );
    }

    #[test]
    fn lights_held_keys_white_and_tested_ones_green() {
        let now = Instant::now();
        let mut tester = Tester::default();
        let first = tester.messages(now);
        assert_eq!(first.len(), LED_COUNT + 2);
        assert_eq!(first[0], "WAKE");
        assert!(lit(&first).is_empty());
        assert_eq!(first[LED_COUNT + 1], protocol::display_message(&tester.picture().hex()));

        tester.board_message(&BoardMessage::Pad {
            index: 0,
            pressed: true,
        });
        assert_eq!(tester.messages(now), ["LED,0,255,255,255"]);
        tester.board_message(&BoardMessage::Pad {
            index: 0,
            pressed: false,
        });
        assert_eq!(tester.messages(now), ["LED,0,0,255,0"]);
        assert!(tester.messages(now).is_empty());
    }

    #[test]
    fn fills_every_led_with_one_colour() {
        let now = Instant::now();
        let mut tester = Tester::default();
        tester.messages(now);
        tester.set_leds(LedTest::Blue);
        let messages = tester.messages(now);
        assert_eq!(messages.len(), LED_COUNT);
        assert!(messages.iter().all(|message| message.ends_with(",0,0,255")));
    }

    #[test]
    fn wakes_the_display_for_a_new_picture_and_while_it_is_looked_at() {
        let now = Instant::now();
        let mut tester = Tester::default();
        tester.messages(now);
        tester.set_display(DisplayTest::Full);
        assert_eq!(
            tester.messages(now),
            ["WAKE".to_string(), protocol::display_message(&tester.picture().hex())]
        );
        assert_eq!(tester.next_update(), Some(now + KEEP_AWAKE));
        assert!(tester.messages(now + KEEP_AWAKE - Duration::from_secs(1)).is_empty());
        assert_eq!(tester.messages(now + KEEP_AWAKE), ["WAKE"]);
    }

    #[test]
    fn sends_everything_again_after_a_reconnection() {
        let now = Instant::now();
        let mut tester = Tester::default();
        tester.board_message(&BoardMessage::Pad {
            index: 0,
            pressed: true,
        });
        tester.board_message(&BoardMessage::EncoderButton { pressed: true });
        tester.messages(now);
        tester.disconnected();
        assert!(!tester.test().held[0] && !tester.test().knob.held);
        assert!(tester.test().tested[0] && tester.test().knob.pressed);
        assert_eq!(tester.next_update(), None);
        assert_eq!(tester.messages(now).len(), LED_COUNT + 2);
    }

    #[test]
    fn restarting_keeps_what_is_shown() {
        let mut tester = Tester::default();
        tester.set_leds(LedTest::Red);
        tester.set_display(DisplayTest::Checkers);
        tester.board_message(&BoardMessage::EncoderButton { pressed: true });
        tester.restart();
        assert_eq!(
            *tester.test(),
            BoardTest {
                leds: LedTest::Red,
                display: DisplayTest::Checkers,
                ..BoardTest::default()
            }
        );
    }

    #[test]
    fn draws_inside_the_screens_margin() {
        let (right, bottom) = (WIDTH - 2, HEIGHT - 2);
        for test in [DisplayTest::Frame, DisplayTest::Full, DisplayTest::Checkers] {
            let canvas = canvas(test);
            let edges = (0..WIDTH).flat_map(|x| [(x, 0), (x, HEIGHT - 1)]);
            let sides = (0..HEIGHT).flat_map(|y| [(0, y), (WIDTH - 1, y)]);
            assert!(edges.chain(sides).all(|(x, y)| !canvas.is_lit(x, y)), "{test:?}");
            assert!(canvas.is_lit(1, 1) && canvas.is_lit(right, bottom), "{test:?}");
        }
        let full = canvas(DisplayTest::Full);
        assert!((1..=right).all(|x| (1..=bottom).all(|y| full.is_lit(x, y))));
        let checkers = canvas(DisplayTest::Checkers);
        assert!(checkers.is_lit(2, 2) && !checkers.is_lit(2, 1) && !checkers.is_lit(1, 2));
    }
}
