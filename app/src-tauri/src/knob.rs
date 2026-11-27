//! The knob's gestures, from its presses, releases and turns.

use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Gesture {
    Turn(i8),
    HoldTurn(i8),
    Tap,
    DoubleTap,
    LongPress,
}

/// The defaults: a second press within this of a tap's release is a double tap; a press held this long, a long press
pub const DOUBLE_TAP: Duration = Duration::from_millis(300);
pub const LONG_PRESS: Duration = Duration::from_millis(600);
/// The ranges the window offers, in milliseconds, for hands that are not so quick
const DOUBLE_TAP_RANGE: (u16, u16) = (200, 1000);
const LONG_PRESS_RANGE: (u16, u16) = (400, 2000);

/// What turning the knob does, plain or held down
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum TurnAction {
    #[default]
    Page,
    Rows,
    SoundboardForOthers,
    SoundboardForMe,
    /// Both soundboard channels together, as one fader
    SoundboardForEveryone,
    PlayingSound,
    Microphone,
    Voice,
    /// The action wheel, while held: let go on a slice to do it
    Wheel,
    Nothing,
}

/// What a tap, a double tap or a long press does
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "action", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PressAction {
    #[default]
    Menu,
    StopSounds,
    MuteMic,
    MuteForMe,
    VoiceChanger,
    HearMyself,
    Background,
    RandomVoice,
    Macro {
        macro_id: String,
    },
    Page {
        page: String,
    },
    /// Hides or shows the overlay while it shows always, else shows it for its time
    Overlay,
    ResetVolumes,
    /// Opens the action wheel, which stays until a press picks or the time runs out
    Wheel,
    Nothing,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnobSettings {
    pub turn: TurnAction,
    pub hold_turn: TurnAction,
    pub tap: PressAction,
    pub double_tap: PressAction,
    pub long_press: PressAction,
    /// How soon after a tap a second one makes a double tap, and how long a long press lasts, in milliseconds
    pub double_tap_ms: u16,
    pub long_press_ms: u16,
}

/// The longest page or macro id a gesture keeps, as a pad (`custom_pages.rs`)
const MAX_TARGET: usize = 64;

impl KnobSettings {
    /// A page or macro id longer than any the app makes or Voicemod uses does nothing
    pub fn checked(self) -> Self {
        let kept = |action: PressAction| match &action {
            PressAction::Macro { macro_id: id } | PressAction::Page { page: id } if id.len() > MAX_TARGET => {
                PressAction::Nothing
            }
            _ => action,
        };
        Self {
            tap: kept(self.tap),
            double_tap: kept(self.double_tap),
            long_press: kept(self.long_press),
            double_tap_ms: self.double_tap_ms.clamp(DOUBLE_TAP_RANGE.0, DOUBLE_TAP_RANGE.1),
            long_press_ms: self.long_press_ms.clamp(LONG_PRESS_RANGE.0, LONG_PRESS_RANGE.1),
            ..self
        }
    }

    pub fn double_tap_time(&self) -> Duration {
        Duration::from_millis(self.double_tap_ms.into())
    }

    pub fn long_press_time(&self) -> Duration {
        Duration::from_millis(self.long_press_ms.into())
    }
}

impl Default for KnobSettings {
    fn default() -> Self {
        Self {
            turn: TurnAction::Page,
            hold_turn: TurnAction::Rows,
            tap: PressAction::Menu,
            double_tap: PressAction::Nothing,
            long_press: PressAction::Nothing,
            double_tap_ms: DOUBLE_TAP.as_millis() as u16,
            long_press_ms: LONG_PRESS.as_millis() as u16,
        }
    }
}

/// Each gesture falls back to its own default when missing or unreadable, so one bad entry loses nothing else.
impl<'de> Deserialize<'de> for KnobSettings {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        let defaults = Self::default();
        Ok(Self {
            turn: field(&value, "turn").unwrap_or(defaults.turn),
            hold_turn: field(&value, "holdTurn").unwrap_or(defaults.hold_turn),
            tap: field(&value, "tap").unwrap_or(defaults.tap),
            double_tap: field(&value, "doubleTap").unwrap_or(defaults.double_tap),
            long_press: field(&value, "longPress").unwrap_or(defaults.long_press),
            double_tap_ms: field(&value, "doubleTapMs").unwrap_or(defaults.double_tap_ms),
            long_press_ms: field(&value, "longPressMs").unwrap_or(defaults.long_press_ms),
        })
    }
}

fn field<T: DeserializeOwned>(value: &Value, name: &str) -> Option<T> {
    T::deserialize(value.get(name)?).ok()
}

#[derive(Default)]
pub struct Gestures {
    /// Whether these have an action: only then a tap waits for a second one, or a long hold is not a tap
    double_tap: bool,
    long_press: bool,
    /// How soon a second tap comes, how long a long press lasts
    times: Option<(Duration, Duration)>,
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

    pub fn set_times(&mut self, double_tap: Duration, long_press: Duration) {
        self.times = Some((double_tap, long_press));
    }

    fn double_tap_time(&self) -> Duration {
        self.times.map_or(DOUBLE_TAP, |(double_tap, _)| double_tap)
    }

    fn long_press_time(&self) -> Duration {
        self.times.map_or(LONG_PRESS, |(_, long_press)| long_press)
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
        let long = now.duration_since(since) >= self.long_press_time();
        // A second press held long is no double tap: the first tap goes, and this press counts on its own
        let mut out = if long { self.let_tap_go() } else { Vec::new() };
        if self.long_press && long {
            out.push(Gesture::LongPress);
            out
        } else if second && !long {
            self.waiting = None;
            vec![Gesture::DoubleTap]
        } else if self.double_tap {
            self.waiting = Some(now + self.double_tap_time());
            out
        } else {
            out.push(Gesture::Tap);
            out
        }
    }

    pub fn turned(&mut self, step: i8) -> Vec<Gesture> {
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
            times: self.times,
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
        assert_eq!(gestures.turned(1), [HoldTurn(1)]);
        assert_eq!(gestures.released(at(start, 200)), []);
    }

    #[test]
    fn turning_while_held_is_not_a_long_press() {
        let start = Instant::now();
        let mut gestures = Gestures::default();
        gestures.set_waits(true, true);
        gestures.pressed(start);
        gestures.turned(-1);
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
        assert_eq!(gestures.turned(1), [Tap, HoldTurn(1)]);
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
    fn a_target_longer_than_any_id_does_nothing() {
        let knob = KnobSettings {
            tap: PressAction::Page { page: "p".repeat(64) },
            double_tap: PressAction::Page { page: "p".repeat(65) },
            long_press: PressAction::Macro {
                macro_id: "m".repeat(1000),
            },
            ..KnobSettings::default()
        };
        let checked = knob.clone().checked();
        assert_eq!(checked.tap, knob.tap);
        assert_eq!(
            (checked.double_tap, checked.long_press),
            (PressAction::Nothing, PressAction::Nothing)
        );
    }

    #[test]
    fn a_long_second_press_is_no_double_tap() {
        let start = Instant::now();
        let mut gestures = Gestures::default();
        gestures.set_waits(true, false);
        tap(&mut gestures, start, 0);
        gestures.pressed(at(start, 100));
        // Held long and let go: the first tap goes, the second is a press of its own
        assert_eq!(gestures.released(at(start, 2400)), [Tap]);
        assert_eq!(gestures.update(at(start, 2700)), [Tap]);
    }

    #[test]
    fn a_turn_right_after_a_tap_comes_after_it() {
        let start = Instant::now();
        let mut gestures = Gestures::default();
        gestures.set_waits(true, false);
        tap(&mut gestures, start, 0);
        assert_eq!(gestures.turned(1), [Tap, Turn(1)]);
        assert_eq!(gestures.next_update(), None);
    }

    #[test]
    fn turning_up_is_a_turn() {
        let mut gestures = Gestures::default();
        assert_eq!(gestures.turned(-1), [Turn(-1)]);
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

    #[test]
    fn slower_hands_get_more_time() {
        let start = Instant::now();
        let mut gestures = Gestures::default();
        gestures.set_waits(true, true);
        gestures.set_times(Duration::from_millis(700), Duration::from_millis(1500));
        assert_eq!(tap(&mut gestures, start, 0), []);
        assert_eq!(tap(&mut gestures, start, 600), [DoubleTap], "a second tap 0.55 s later");
        gestures.pressed(at(start, 3000));
        assert_eq!(
            gestures.released(at(start, 3800)),
            [],
            "0.8 s is still a tap, waiting for a second one"
        );
        assert_eq!(
            gestures.pressed(at(start, 6000)),
            [Tap],
            "its window over, the tap goes"
        );
        assert_eq!(gestures.released(at(start, 7500)), [LongPress], "held 1.5 s");
    }

    #[test]
    fn the_times_stay_within_the_window_s_ranges() {
        let knob = KnobSettings {
            double_tap_ms: 50,
            long_press_ms: 9000,
            ..KnobSettings::default()
        }
        .checked();
        assert_eq!((knob.double_tap_ms, knob.long_press_ms), (200, 2000));
    }

    #[test]
    fn the_defaults_are_todays_knob() {
        let knob = KnobSettings::default();
        assert_eq!((knob.turn, knob.hold_turn), (TurnAction::Page, TurnAction::Rows));
        assert_eq!(
            (knob.tap, knob.double_tap, knob.long_press),
            (PressAction::Menu, PressAction::Nothing, PressAction::Nothing)
        );
    }

    #[test]
    fn each_gesture_falls_back_alone() {
        let knob: KnobSettings = serde_json::from_str(
            r#"{"turn": {"action": "dance"}, "tap": {"action": "macro"}, "longPress": {"action": "muteMic"}, "holdTurn": 3}"#,
        )
        .unwrap();
        assert_eq!(knob.turn, TurnAction::Page);
        assert_eq!(knob.tap, PressAction::Menu, "a macro action without its macro");
        assert_eq!(knob.long_press, PressAction::MuteMic);
        assert_eq!(knob.hold_turn, TurnAction::Rows);
        assert_eq!(knob.double_tap, PressAction::Nothing);
        for broken in ["null", "[]", r#""menu""#] {
            assert_eq!(
                serde_json::from_str::<KnobSettings>(broken).unwrap(),
                KnobSettings::default()
            );
        }
    }

    #[test]
    fn knob_settings_round_trip() {
        let knob = KnobSettings {
            turn: TurnAction::SoundboardForOthers,
            hold_turn: TurnAction::PlayingSound,
            tap: PressAction::Macro {
                macro_id: "macro-1".into(),
            },
            double_tap: PressAction::Page {
                page: "drezimo-page-2".into(),
            },
            long_press: PressAction::ResetVolumes,
            double_tap_ms: 450,
            long_press_ms: 1200,
        };
        let json = serde_json::to_value(&knob).unwrap();
        assert_eq!(
            json["tap"],
            serde_json::json!({"action": "macro", "macroId": "macro-1"})
        );
        assert_eq!(
            json["doubleTap"],
            serde_json::json!({"action": "page", "page": "drezimo-page-2"})
        );
        assert_eq!(json["turn"], serde_json::json!({"action": "soundboardForOthers"}));
        assert_eq!(serde_json::from_value::<KnobSettings>(json).unwrap(), knob);
    }
}
