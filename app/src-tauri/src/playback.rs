//! What Voicemod is playing. Each press is replayed at once with Voicemod's own rules (docs/project.md, section 8);
//! the log, which reaches the disk about a second later, then confirms it or corrects it. Without the log, a sound's
//! length is a guess.

use crate::soundboard::{PlaybackMode, Sound};
use crate::voicemod_log::LogEvent;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

const GUESSED_LENGTH: Duration = Duration::from_secs(3);
/// How far apart in time a prediction and the log line that confirms it can be
const CONFIRM_WINDOW: Duration = Duration::from_millis(600);
/// A prediction the log has not confirmed by then means the state drifted: it is taken from the log again
const CONFIRM_TIMEOUT: Duration = Duration::from_secs(4);
/// Predictions left unconfirmed in a row before the log is taken as unreadable, as after a Voicemod update that
/// changed its format
const MISSES_TO_DISTRUST: u32 = 3;
/// A change made here this long after a log line is newer than what the line tells
const NEWER_THAN_LINE: Duration = Duration::from_millis(150);
const NEWER_THAN_STOP: Duration = Duration::from_millis(50);
/// A sound still playing this long after its known end is an orphan that Voicemod never removed: it is silent
const ORPHAN_AFTER: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Playing,
    /// Still active in Voicemod: the next press resumes it
    Paused,
}

struct Active {
    state: State,
    /// When a playing sound ends by itself, if known
    ends_at: Option<Instant>,
    /// How much a paused sound had left
    remaining: Option<Duration>,
}

impl Active {
    fn playing(ends_at: Option<Instant>) -> Self {
        Self {
            state: State::Playing,
            ends_at,
            remaining: None,
        }
    }

    fn pause(&mut self, now: Instant) {
        self.state = State::Paused;
        self.remaining = self.ends_at.take().map(|end| end.saturating_duration_since(now));
    }

    fn resume(&mut self, now: Instant) {
        self.state = State::Playing;
        self.ends_at = self.remaining.take().map(|left| now + left);
    }
}

/// A sound the log reports active
pub struct LoggedSound<'a> {
    pub id: &'a str,
    pub sound: Option<&'a Sound>,
    pub state: State,
    /// When it would have started, had it never been paused
    pub started: Instant,
}

/// A log line a press should produce
#[derive(PartialEq)]
enum Line {
    Play { id: String, repeat: bool },
    End { id: String },
    StopAll,
}

struct Expected {
    line: Line,
    at: Instant,
}

#[derive(Default)]
pub struct Playback {
    sounds: HashMap<String, Active>,
    /// When each sound last changed here
    changed: HashMap<String, Instant>,
    /// Natural lengths, learned from the log
    lengths: HashMap<String, Duration>,
    /// The log lines the presses should produce, while the log is followed
    expected: Option<Vec<Expected>>,
    /// Predictions the log left unconfirmed since it last confirmed one
    misses: u32,
}

impl Playback {
    /// Starts or stops following the log. What plays is unknown again either way.
    pub fn follow_log(&mut self, following: bool) {
        self.reset();
        self.expected = following.then(Vec::new);
        self.misses = 0;
    }

    pub fn follows_log(&self) -> bool {
        self.expected.is_some()
    }

    pub fn learn_length(&mut self, id: &str, length: Duration) {
        self.lengths.insert(id.to_string(), length);
    }

    pub fn press(&mut self, sound: &Sound, now: Instant) {
        let id = &sound.id;
        let repeat = self.sounds.contains_key(id);
        if !repeat && sound.stop_other_sounds {
            let others: Vec<String> = self.sounds.keys().cloned().collect();
            for other in &others {
                self.stop(other, now);
            }
        }
        self.expect(Line::Play { id: id.clone(), repeat }, now);
        self.changed.insert(id.clone(), now);

        let natural_end = self.natural_end(Some(sound), id, now);
        let Some(active) = self.sounds.get_mut(id) else {
            self.sounds.insert(id.clone(), Active::playing(natural_end));
            return;
        };
        match (active.state, sound.playback_mode) {
            (State::Paused, _) => active.resume(now),
            (_, PlaybackMode::PlayPause) => active.pause(now),
            (_, PlaybackMode::PlayRestart) => active.ends_at = natural_end,
            (_, PlaybackMode::PlayOverlap) => active.ends_at = active.ends_at.max(natural_end),
            _ => self.stop(id, now),
        }
    }

    /// The key of a sound that plays while held was released.
    pub fn release(&mut self, sound: &Sound, now: Instant) {
        if self.sounds.contains_key(&sound.id) {
            // Voicemod logs the key-up as a repeat press, then the end
            self.expect(
                Line::Play {
                    id: sound.id.clone(),
                    repeat: true,
                },
                now,
            );
            self.stop(&sound.id, now);
        }
    }

    pub fn stop_all(&mut self, now: Instant) {
        self.expect(Line::StopAll, now);
        let ids: Vec<String> = self.sounds.keys().cloned().collect();
        for id in &ids {
            self.stop(id, now);
        }
    }

    /// Forgets what plays: Voicemod went away or came back.
    pub fn reset(&mut self) {
        self.sounds.clear();
        self.changed.clear();
        if let Some(expected) = &mut self.expected {
            expected.clear();
        }
    }

    /// Ends the sounds whose time is up. True when a prediction went unconfirmed by the log: the state should then
    /// be taken from the log again.
    pub fn update(&mut self, now: Instant) -> bool {
        let ended: Vec<(String, Instant)> = self
            .sounds
            .iter()
            .filter_map(|(id, active)| Some((id.clone(), active.ends_at.filter(|end| *end <= now)?)))
            .collect();
        for (id, end) in ended {
            self.stop(&id, end);
        }

        let Some(expected) = &mut self.expected else {
            return false;
        };
        let before = expected.len();
        expected.retain(|expected| now.saturating_duration_since(expected.at) < CONFIRM_TIMEOUT);
        let missed = (before - expected.len()) as u32;
        self.misses += missed;
        if self.misses >= MISSES_TO_DISTRUST {
            eprintln!("Voicemod's log no longer confirms the presses: guessing the playback instead");
            self.follow_log(false);
            return false;
        }
        missed > 0
    }

    /// When `update` has something to do.
    pub fn next_change(&self) -> Option<Instant> {
        let ends = self.sounds.values().filter_map(|active| active.ends_at);
        let timeouts = self
            .expected
            .iter()
            .flatten()
            .map(|expected| expected.at + CONFIRM_TIMEOUT);
        ends.chain(timeouts).min()
    }

    /// Applies a log line written at `at`; `sound` is the sound's metadata, when known.
    pub fn apply(&mut self, event: &LogEvent, at: Instant, sound: Option<&Sound>) {
        if self.confirm(event, at) {
            return;
        }
        match event {
            LogEvent::StoppedAll | LogEvent::Restarted => self.forget_all_but(None, at),
            LogEvent::StoppedOthers { playing } => self.forget_all_but(Some(playing), at),
            // A change made here after the line wins: the line tells an older moment
            LogEvent::Played { id, .. } | LogEvent::Ended { id, .. } if self.changed_after(id, at, NEWER_THAN_LINE) => {
            }
            LogEvent::Ended { id, left: None, .. } => {
                self.forget(id, at);
            }
            LogEvent::Ended {
                id, left: Some(state), ..
            } => self.take(id, *state, false, at, sound),
            LogEvent::Played {
                id, from_start, state, ..
            } => self.take(id, *state, *from_start, at, sound),
        }
    }

    /// Takes the state from the log, as of its last line, except where a press changed it since.
    pub fn resync(&mut self, logged: &[LoggedSound], as_of: Instant, now: Instant) {
        let ids: HashSet<String> = self
            .sounds
            .keys()
            .cloned()
            .chain(logged.iter().map(|sound| sound.id.to_string()))
            .collect();
        for id in ids {
            if self.changed_after(&id, as_of, NEWER_THAN_STOP) {
                continue;
            }
            let Some(logged) = logged.iter().find(|sound| sound.id == id) else {
                self.forget(&id, as_of);
                continue;
            };
            let natural_end = self.natural_end(logged.sound, &id, logged.started);
            let playing = logged.state == State::Playing;
            if playing && natural_end.is_some_and(|end| end + ORPHAN_AFTER < now) {
                self.forget(&id, as_of);
                continue;
            }
            let active = Active {
                state: logged.state,
                ends_at: natural_end.filter(|_| playing),
                remaining: None,
            };
            self.sounds.insert(id.clone(), active);
            self.changed.insert(id, as_of);
        }
        if let Some(expected) = &mut self.expected {
            expected.retain(|expected| expected.at > as_of);
        }
    }

    pub fn state(&self, id: &str) -> Option<State> {
        self.sounds.get(id).map(|active| active.state)
    }

    pub fn is_empty(&self) -> bool {
        self.sounds.is_empty()
    }

    /// Takes the state the log tells for a sound that changed without a prediction, such as one played from
    /// Voicemod's own window.
    fn take(&mut self, id: &str, state: State, from_start: bool, at: Instant, sound: Option<&Sound>) {
        let natural_end = self.natural_end(sound, id, at);
        let previous = self.state(id);
        let active = self
            .sounds
            .entry(id.to_string())
            .or_insert_with(|| Active::playing(natural_end));
        if from_start {
            active.ends_at = active.ends_at.max(natural_end);
            active.remaining = None;
        }
        match (previous, state) {
            (Some(State::Paused), State::Playing) => active.resume(at),
            (Some(State::Playing) | None, State::Paused) => active.pause(at),
            _ => {}
        }
        self.changed.insert(id.to_string(), at);
    }

    /// Whether the line is one a prediction expected, which it then confirms.
    fn confirm(&mut self, event: &LogEvent, at: Instant) -> bool {
        let Some(expected) = &mut self.expected else {
            return false;
        };
        let line = match event {
            LogEvent::Played { id, repeat, .. } => Line::Play {
                id: id.clone(),
                repeat: *repeat,
            },
            LogEvent::Ended { id, .. } => Line::End { id: id.clone() },
            LogEvent::StoppedAll => Line::StopAll,
            LogEvent::StoppedOthers { .. } | LogEvent::Restarted => return false,
        };
        let close = |when: Instant| when.max(at).duration_since(when.min(at)) < CONFIRM_WINDOW;
        let Some(index) = expected
            .iter()
            .position(|expected| expected.line == line && close(expected.at))
        else {
            return false;
        };
        expected.remove(index);
        self.misses = 0;
        true
    }

    fn expect(&mut self, line: Line, at: Instant) {
        if let Some(expected) = &mut self.expected {
            expected.push(Expected { line, at });
        }
    }

    fn stop(&mut self, id: &str, at: Instant) {
        if self.forget(id, at) {
            self.expect(Line::End { id: id.to_string() }, at);
        }
    }

    fn forget(&mut self, id: &str, at: Instant) -> bool {
        self.changed.insert(id.to_string(), at);
        self.sounds.remove(id).is_some()
    }

    /// Forgets every sound but `kept`, except the ones a press changed after `at`.
    fn forget_all_but(&mut self, kept: Option<&str>, at: Instant) {
        let stopped: Vec<String> = self
            .sounds
            .keys()
            .filter(|id| Some(id.as_str()) != kept && !self.changed_after(id, at, NEWER_THAN_STOP))
            .cloned()
            .collect();
        for id in &stopped {
            self.forget(id, at);
        }
    }

    fn changed_after(&self, id: &str, at: Instant, margin: Duration) -> bool {
        self.changed.get(id).is_some_and(|changed| *changed > at + margin)
    }

    fn natural_end(&self, sound: Option<&Sound>, id: &str, from: Instant) -> Option<Instant> {
        let endless = sound.is_some_and(|sound| sound.looped || sound.playback_mode == PlaybackMode::PlayLoopOnPress);
        if endless {
            return None;
        }
        match self.lengths.get(id) {
            Some(length) => Some(from + *length),
            // The log will tell the real end
            None if self.follows_log() => None,
            None => Some(from + GUESSED_LENGTH),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::soundboard::tests::sound;

    const MS: Duration = Duration::from_millis(1);

    fn with_mode(id: &str, playback_mode: PlaybackMode) -> Sound {
        Sound {
            playback_mode,
            ..sound(id)
        }
    }

    fn following() -> Playback {
        let mut playback = Playback::default();
        playback.follow_log(true);
        playback
    }

    fn played(id: &str, repeat: bool, state: State) -> LogEvent {
        LogEvent::Played {
            id: id.into(),
            repeat,
            from_start: !repeat,
            state,
        }
    }

    fn ended(id: &str) -> LogEvent {
        LogEvent::Ended {
            id: id.into(),
            left: None,
            length: None,
        }
    }

    #[test]
    fn a_sound_plays_until_its_end() {
        let mut playback = Playback::default();
        let start = Instant::now();
        playback.press(&sound("a"), start);
        assert_eq!(playback.state("a"), Some(State::Playing));
        assert_eq!(playback.next_change(), Some(start + GUESSED_LENGTH));

        playback.update(start + GUESSED_LENGTH);
        assert_eq!(playback.state("a"), None);
        assert!(playback.is_empty());
    }

    #[test]
    fn restart_starts_the_length_again() {
        let mut playback = Playback::default();
        let start = Instant::now();
        let later = start + Duration::from_secs(2);
        playback.press(&sound("a"), start);
        playback.press(&sound("a"), later);
        assert_eq!(playback.next_change(), Some(later + GUESSED_LENGTH));
    }

    #[test]
    fn pause_keeps_the_time_left() {
        let mut playback = Playback::default();
        let pausing = with_mode("a", PlaybackMode::PlayPause);
        let start = Instant::now();
        playback.press(&pausing, start);
        playback.press(&pausing, start + Duration::from_secs(1));
        assert_eq!(playback.state("a"), Some(State::Paused));
        assert_eq!(playback.next_change(), None);

        let resumed = start + Duration::from_secs(60);
        playback.update(resumed);
        playback.press(&pausing, resumed);
        assert_eq!(playback.state("a"), Some(State::Playing));
        assert_eq!(playback.next_change(), Some(resumed + Duration::from_secs(2)));
    }

    #[test]
    fn play_stop_toggles() {
        let mut playback = Playback::default();
        let toggling = with_mode("a", PlaybackMode::PlayStop);
        let now = Instant::now();
        playback.press(&toggling, now);
        playback.press(&toggling, now);
        assert_eq!(playback.state("a"), None);
    }

    #[test]
    fn loops_and_held_sounds_never_end_by_themselves() {
        let mut playback = Playback::default();
        let looped = Sound {
            looped: true,
            ..sound("loop")
        };
        let held = with_mode("held", PlaybackMode::PlayLoopOnPress);
        let now = Instant::now();
        playback.press(&looped, now);
        playback.press(&held, now);
        playback.update(now + Duration::from_secs(3600));
        assert_eq!(playback.state("loop"), Some(State::Playing));

        playback.release(&held, now);
        assert_eq!(playback.state("held"), None);
    }

    #[test]
    fn a_sound_can_stop_the_others() {
        let mut playback = Playback::default();
        let now = Instant::now();
        playback.press(&sound("a"), now);
        playback.press(
            &Sound {
                stop_other_sounds: true,
                ..sound("b")
            },
            now,
        );
        assert_eq!(playback.state("a"), None);
        assert_eq!(playback.state("b"), Some(State::Playing));

        playback.stop_all(now);
        assert!(playback.is_empty());
    }

    #[test]
    fn the_log_confirms_predictions_and_tells_the_real_end() {
        let mut playback = following();
        let start = Instant::now();
        playback.press(&sound("a"), start);
        assert_eq!(
            playback.next_change(),
            Some(start + CONFIRM_TIMEOUT),
            "no guessed end with the log"
        );

        playback.apply(&played("a", false, State::Playing), start + 300 * MS, None);
        assert_eq!(playback.state("a"), Some(State::Playing));
        assert_eq!(playback.next_change(), None, "confirmed");

        playback.apply(&ended("a"), start + 900 * MS, None);
        assert_eq!(playback.state("a"), None);
    }

    #[test]
    fn learned_lengths_predict_the_end() {
        let mut playback = following();
        playback.learn_length("a", 850 * MS);
        let start = Instant::now();
        playback.press(&sound("a"), start);
        playback.update(start + 850 * MS);
        assert_eq!(playback.state("a"), None);

        playback.apply(&played("a", false, State::Playing), start + 100 * MS, None);
        playback.apply(&ended("a"), start + 900 * MS, None);
        assert!(!playback.update(start + 900 * MS), "both lines were expected");
        assert_eq!(playback.state("a"), None);
    }

    #[test]
    fn follows_sounds_played_from_voicemod() {
        let mut playback = following();
        playback.learn_length("a", Duration::from_secs(5));
        let now = Instant::now();
        playback.apply(&played("a", false, State::Playing), now, None);
        assert_eq!(playback.state("a"), Some(State::Playing));
        assert_eq!(playback.next_change(), Some(now + Duration::from_secs(5)));

        let restarted = LogEvent::Played {
            id: "a".into(),
            repeat: true,
            from_start: true,
            state: State::Playing,
        };
        playback.apply(&restarted, now + Duration::from_secs(4), None);
        assert_eq!(playback.next_change(), Some(now + Duration::from_secs(9)));

        playback.apply(&played("a", true, State::Paused), now + Duration::from_secs(5), None);
        assert_eq!(playback.state("a"), Some(State::Paused));

        playback.apply(&LogEvent::Restarted, now + Duration::from_secs(6), None);
        assert!(playback.is_empty());
    }

    #[test]
    fn a_sound_that_stops_the_others_leaves_only_itself() {
        let mut playback = following();
        let now = Instant::now();
        playback.apply(&played("a", false, State::Playing), now, None);
        playback.apply(&played("b", false, State::Playing), now, None);
        let others = LogEvent::StoppedOthers { playing: "b".into() };
        playback.apply(&others, now + 500 * MS, None);
        assert_eq!(playback.state("a"), None);
        assert_eq!(playback.state("b"), Some(State::Playing));
    }

    #[test]
    fn a_late_line_does_not_undo_a_newer_press() {
        let mut playback = following();
        let start = Instant::now();
        playback.press(&sound("a"), start + Duration::from_secs(2));
        playback.apply(&ended("a"), start, None);
        assert_eq!(playback.state("a"), Some(State::Playing));
    }

    #[test]
    fn an_unconfirmed_prediction_asks_for_a_resync() {
        let mut playback = following();
        let start = Instant::now();
        playback.press(&sound("a"), start);
        assert!(!playback.update(start + Duration::from_secs(3)));
        assert!(playback.update(start + CONFIRM_TIMEOUT));

        playback.resync(&[], start + CONFIRM_TIMEOUT, start + CONFIRM_TIMEOUT);
        assert!(playback.is_empty(), "the log shows nothing playing");
    }

    #[test]
    fn stops_trusting_a_log_that_never_confirms() {
        let mut playback = following();
        let start = Instant::now();
        for (i, id) in ["a", "b", "c"].into_iter().enumerate() {
            playback.press(&sound(id), start + i as u32 * MS);
        }
        playback.update(start + CONFIRM_TIMEOUT + 10 * MS);
        assert!(!playback.follows_log());
        assert!(playback.is_empty());

        playback.press(&sound("a"), start + CONFIRM_TIMEOUT);
        assert_eq!(playback.next_change(), Some(start + CONFIRM_TIMEOUT + GUESSED_LENGTH));
    }

    #[test]
    fn resync_takes_the_log_state_but_drops_orphans() {
        let mut playback = following();
        playback.learn_length("orphan", Duration::from_secs(1));
        let now = Instant::now();
        let long_ago = now - Duration::from_secs(60);
        let logged = [
            LoggedSound {
                id: "loop",
                sound: None,
                state: State::Paused,
                started: long_ago,
            },
            LoggedSound {
                id: "orphan",
                sound: None,
                state: State::Playing,
                started: long_ago,
            },
        ];
        playback.resync(&logged, now, now);
        assert_eq!(playback.state("loop"), Some(State::Paused));
        assert_eq!(playback.state("orphan"), None);
    }
}
