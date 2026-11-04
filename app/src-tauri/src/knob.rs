//! The knob's gestures, from its presses, releases and turns.

use std::time::{Duration, Instant};

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Gesture {
    Turn(i8),
    HoldTurn(i8),
    Tap,
    DoubleTap,
    LongPress,
}

pub const DOUBLE_TAP: Duration = Duration::from_millis(300);
pub const LONG_PRESS: Duration = Duration::from_millis(600);

#[derive(Default)]
pub struct Gestures {
    /// Whether these have an action: only then a tap waits for a second one, or a long hold is not a tap
    double_tap: bool,
    long_press: bool,
    /// Since when the knob is down
    down: Option<Instant>,
    /// Turned, or held for a peek, since it went down: its release is no gesture
    used: bool,
    /// A tap waiting for a second one until then
    waiting: Option<Instant>,
    /// The press under way came while a tap was waiting
    second: bool,
}

impl Gestures {
    pub fn set_waits(&mut self, double_tap: bool, long_press: bool) {
        self.double_tap = double_tap;
        self.long_press = long_press;
    }

    pub fn pressed(&mut self, now: Instant) -> Vec<Gesture> {
        if self.down.is_some() {
            return Vec::new();
        }
        let out = self.update(now);
        self.second = self.waiting.is_some();
        self.down = Some(now);
        self.used = false;
        out
    }

    pub fn released(&mut self, now: Instant) -> Vec<Gesture> {
        let Some(since) = self.down.take() else {
            return Vec::new();
        };
        let second = std::mem::take(&mut self.second);
        if self.used {
            return Vec::new();
        }
        if self.long_press && now.duration_since(since) >= LONG_PRESS {
            let mut out = self.let_tap_go();
            out.push(Gesture::LongPress);
            out
        } else if second {
            self.waiting = None;
            vec![Gesture::DoubleTap]
        } else if self.double_tap {
            self.waiting = Some(now + DOUBLE_TAP);
            Vec::new()
        } else {
            vec![Gesture::Tap]
        }
    }

    pub fn turned(&mut self, step: i8, _now: Instant) -> Vec<Gesture> {
        let mut out = self.let_tap_go();
        if self.down.is_some() {
            self.used = true;
            out.push(Gesture::HoldTurn(step));
        } else {
            out.push(Gesture::Turn(step));
        }
        out
    }

    /// A pad pressed while the knob is down: the press is a peek, not a tap
    pub fn pad_pressed(&mut self) -> Vec<Gesture> {
        if self.down.is_none() {
            return Vec::new();
        }
        self.used = true;
        self.let_tap_go()
    }

    pub fn is_down(&self) -> bool {
        self.down.is_some()
    }

    pub fn next_update(&self) -> Option<Instant> {
        // A second press decides on its release
        if self.second { None } else { self.waiting }
    }

    /// A tap whose double-tap window ended
    pub fn update(&mut self, now: Instant) -> Vec<Gesture> {
        match self.next_update() {
            Some(until) if until <= now => self.let_tap_go(),
            _ => Vec::new(),
        }
    }

    /// Forgets a press in progress, as when the board goes away
    pub fn reset(&mut self) {
        *self = Self {
            double_tap: self.double_tap,
            long_press: self.long_press,
            ..Self::default()
        };
    }

    fn let_tap_go(&mut self) -> Vec<Gesture> {
        self.waiting.take().map(|_| Gesture::Tap).into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::Gesture::*;
    use super::*;

    fn at(start: Instant, millis: u64) -> Instant {
        start + Duration::from_millis(millis)
    }

    fn tap(gestures: &mut Gestures, start: Instant, millis: u64) -> Vec<Gesture> {
        let mut out = gestures.pressed(at(start, millis));
        out.extend(gestures.released(at(start, millis + 50)));
        out
    }

    #[test]
    fn without_double_tap_a_tap_acts_at_once() {
        let start = Instant::now();
        let mut gestures = Gestures::default();
        assert_eq!(tap(&mut gestures, start, 0), [Tap]);
        assert_eq!(gestures.next_update(), None);
    }

    #[test]
    fn a_long_hold_is_a_tap_while_long_press_does_nothing() {
        let start = Instant::now();
        let mut gestures = Gestures::default();
        gestures.pressed(start);
        assert!(gestures.is_down());
        assert_eq!(gestures.released(at(start, 2000)), [Tap]);
        assert!(!gestures.is_down());
    }

    #[test]
    fn a_long_press_once_it_has_an_action() {
        let start = Instant::now();
        let mut gestures = Gestures::default();
        gestures.set_waits(false, true);
        gestures.pressed(start);
        assert_eq!(gestures.released(at(start, 599)), [Tap]);
        gestures.pressed(at(start, 1000));
        assert_eq!(gestures.released(at(start, 1600)), [LongPress]);
    }

    #[test]
    fn a_double_tap_within_300_ms() {
        let start = Instant::now();
        let mut gestures = Gestures::default();
        gestures.set_waits(true, false);
        assert_eq!(tap(&mut gestures, start, 0), []);
        assert_eq!(tap(&mut gestures, start, 250), [DoubleTap]);
        assert_eq!(gestures.next_update(), None);
        assert_eq!(gestures.update(at(start, 1000)), []);
    }

    #[test]
    fn a_double_tap_counts_from_the_second_press() {
        let start = Instant::now();
        let mut gestures = Gestures::default();
        gestures.set_waits(true, false);
        tap(&mut gestures, start, 0);
        gestures.pressed(at(start, 300));
        // The window ended while the knob was down: the release still makes it a double tap
        assert_eq!(gestures.update(at(start, 400)), []);
        assert_eq!(gestures.released(at(start, 420)), [DoubleTap]);
    }

    #[test]
    fn a_single_tap_waits_for_the_window_then_acts() {
        let start = Instant::now();
        let mut gestures = Gestures::default();
        gestures.set_waits(true, false);
        assert_eq!(tap(&mut gestures, start, 0), []);
        assert_eq!(gestures.next_update(), Some(at(start, 350)));
        assert_eq!(gestures.update(at(start, 349)), []);
        assert_eq!(gestures.update(at(start, 350)), [Tap]);
        assert_eq!(gestures.next_update(), None);
    }

    #[test]
    fn a_press_after_the_window_lets_the_first_tap_go() {
        let start = Instant::now();
        let mut gestures = Gestures::default();
        gestures.set_waits(true, false);
        tap(&mut gestures, start, 0);
        // No update came in between
        assert_eq!(gestures.pressed(at(start, 500)), [Tap]);
        assert_eq!(gestures.released(at(start, 550)), []);
        assert_eq!(gestures.update(at(start, 850)), [Tap]);
    }

    #[test]
    fn turning_while_held_is_not_a_tap() {
        let start = Instant::now();
        let mut gestures = Gestures::default();
        gestures.pressed(start);
        assert_eq!(gestures.turned(1, at(start, 100)), [HoldTurn(1)]);
        assert_eq!(gestures.released(at(start, 200)), []);
    }

    #[test]
    fn turning_while_held_is_not_a_long_press() {
        let start = Instant::now();
        let mut gestures = Gestures::default();
        gestures.set_waits(true, true);
        gestures.pressed(start);
        gestures.turned(-1, at(start, 100));
        assert_eq!(gestures.released(at(start, 2000)), []);
        assert_eq!(gestures.next_update(), None);
    }

    #[test]
    fn a_peek_is_not_a_tap() {
        let start = Instant::now();
        let mut gestures = Gestures::default();
        gestures.pressed(start);
        assert_eq!(gestures.pad_pressed(), []);
        assert_eq!(gestures.released(at(start, 100)), []);
    }

    #[test]
    fn a_second_press_that_turns_lets_the_first_tap_go() {
        let start = Instant::now();
        let mut gestures = Gestures::default();
        gestures.set_waits(true, false);
        tap(&mut gestures, start, 0);
        assert_eq!(gestures.pressed(at(start, 100)), []);
        assert_eq!(gestures.turned(1, at(start, 150)), [Tap, HoldTurn(1)]);
        assert_eq!(gestures.released(at(start, 200)), []);
        assert_eq!(gestures.update(at(start, 1000)), []);
    }

    #[test]
    fn a_second_press_that_peeks_lets_the_first_tap_go() {
        let start = Instant::now();
        let mut gestures = Gestures::default();
        gestures.set_waits(true, false);
        tap(&mut gestures, start, 0);
        gestures.pressed(at(start, 100));
        assert_eq!(gestures.pad_pressed(), [Tap]);
        assert_eq!(gestures.released(at(start, 200)), []);
    }

    #[test]
    fn a_second_press_held_long_lets_the_first_tap_go() {
        let start = Instant::now();
        let mut gestures = Gestures::default();
        gestures.set_waits(true, true);
        tap(&mut gestures, start, 0);
        gestures.pressed(at(start, 100));
        assert_eq!(gestures.released(at(start, 700)), [Tap, LongPress]);
    }

    #[test]
    fn a_turn_right_after_a_tap_comes_after_it() {
        let start = Instant::now();
        let mut gestures = Gestures::default();
        gestures.set_waits(true, false);
        tap(&mut gestures, start, 0);
        assert_eq!(gestures.turned(1, at(start, 120)), [Tap, Turn(1)]);
        assert_eq!(gestures.next_update(), None);
    }

    #[test]
    fn turning_up_is_a_turn() {
        let start = Instant::now();
        let mut gestures = Gestures::default();
        assert_eq!(gestures.turned(-1, start), [Turn(-1)]);
    }

    #[test]
    fn a_reset_forgets_the_press() {
        let start = Instant::now();
        let mut gestures = Gestures::default();
        gestures.set_waits(true, false);
        tap(&mut gestures, start, 0);
        gestures.pressed(at(start, 100));
        gestures.reset();
        assert!(!gestures.is_down());
        assert_eq!(gestures.released(at(start, 150)), []);
        assert_eq!(gestures.next_update(), None);
        // What the knob does stays set
        assert_eq!(tap(&mut gestures, start, 1000), []);
    }
}
