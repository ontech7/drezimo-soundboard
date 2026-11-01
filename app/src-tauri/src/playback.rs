//! What Voicemod is playing. Its API does not say, so each press is replayed with Voicemod's own rules
//! (docs/project.md, section 8); until the log watcher reports the real lengths, a sound's length is a guess.

use crate::soundboard::{PlaybackMode, Sound};
use std::collections::HashMap;
use std::time::{Duration, Instant};

const GUESSED_LENGTH: Duration = Duration::from_secs(3);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Playing,
    /// Still active in Voicemod: the next press resumes it
    Paused,
}

struct Active {
    state: State,
    /// When a playing sound ends by itself; never for loops
    ends_at: Option<Instant>,
    /// How much a paused sound had left
    remaining: Option<Duration>,
}

#[derive(Default)]
pub struct Playback {
    sounds: HashMap<String, Active>,
}

impl Playback {
    pub fn press(&mut self, sound: &Sound, now: Instant) {
        let Some(active) = self.sounds.get_mut(&sound.id) else {
            if sound.stop_other_sounds {
                self.sounds.clear();
            }
            let started = Active {
                state: State::Playing,
                ends_at: natural_end(sound, now),
                remaining: None,
            };
            self.sounds.insert(sound.id.clone(), started);
            return;
        };

        if active.state == State::Paused {
            active.state = State::Playing;
            active.ends_at = active.remaining.take().map(|left| now + left);
            return;
        }
        match sound.playback_mode {
            PlaybackMode::PlayPause => {
                active.state = State::Paused;
                active.remaining = active.ends_at.take().map(|end| end.saturating_duration_since(now));
            }
            PlaybackMode::PlayRestart => active.ends_at = natural_end(sound, now),
            PlaybackMode::PlayOverlap => active.ends_at = active.ends_at.max(natural_end(sound, now)),
            PlaybackMode::PlayStop | PlaybackMode::PlayLoopOnPress | PlaybackMode::Unknown => {
                self.sounds.remove(&sound.id);
            }
        }
    }

    /// The key of a sound that plays while held was released.
    pub fn release(&mut self, sound: &Sound) {
        self.sounds.remove(&sound.id);
    }

    pub fn stop_all(&mut self) {
        self.sounds.clear();
    }

    /// Forgets the sounds that have ended by `now`.
    pub fn update(&mut self, now: Instant) {
        self.sounds
            .retain(|_, active| active.ends_at.is_none_or(|end| end > now));
    }

    pub fn state(&self, id: &str) -> Option<State> {
        self.sounds.get(id).map(|active| active.state)
    }

    pub fn is_empty(&self) -> bool {
        self.sounds.is_empty()
    }

    pub fn next_end(&self) -> Option<Instant> {
        self.sounds.values().filter_map(|active| active.ends_at).min()
    }
}

fn natural_end(sound: &Sound, from: Instant) -> Option<Instant> {
    let endless = sound.looped || sound.playback_mode == PlaybackMode::PlayLoopOnPress;
    (!endless).then(|| from + GUESSED_LENGTH)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::soundboard::tests::sound;

    fn with_mode(id: &str, playback_mode: PlaybackMode) -> Sound {
        Sound {
            playback_mode,
            ..sound(id)
        }
    }

    #[test]
    fn a_sound_plays_until_its_end() {
        let mut playback = Playback::default();
        let start = Instant::now();
        playback.press(&sound("a"), start);
        assert_eq!(playback.state("a"), Some(State::Playing));
        assert_eq!(playback.next_end(), Some(start + GUESSED_LENGTH));

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
        assert_eq!(playback.next_end(), Some(later + GUESSED_LENGTH));
    }

    #[test]
    fn pause_keeps_the_time_left() {
        let mut playback = Playback::default();
        let pausing = with_mode("a", PlaybackMode::PlayPause);
        let start = Instant::now();
        playback.press(&pausing, start);
        playback.press(&pausing, start + Duration::from_secs(1));
        assert_eq!(playback.state("a"), Some(State::Paused));
        assert_eq!(playback.next_end(), None);

        playback.update(start + Duration::from_secs(60));
        assert_eq!(playback.state("a"), Some(State::Paused));

        let resumed = start + Duration::from_secs(60);
        playback.press(&pausing, resumed);
        assert_eq!(playback.state("a"), Some(State::Playing));
        assert_eq!(playback.next_end(), Some(resumed + Duration::from_secs(2)));
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

        playback.release(&held);
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

        playback.stop_all();
        assert!(playback.is_empty());
    }
}
