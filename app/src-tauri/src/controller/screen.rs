//! What the display shows.

use super::{COLUMNS, Controller, ROWS};
use crate::display::screens::{self, FRAME, Progress, Screen, SoundState, Switch};
use crate::playback::{GUESSED_LENGTH, State};
use crate::soundboard::{FAVOURITES_ID, Pad, Page, PlaybackMode, VOICES_ID};
use crate::voicemod::VoicemodStatus;
use std::time::{Duration, Instant};

/// How long the last sound's name stays on the display
pub(super) const LAST_SOUND_TIME: Duration = Duration::from_secs(8);
/// About the bar's length in pixels: it moves no more often than that many times a sound
const BAR_STEPS: u32 = 110;
const TENTH: Duration = Duration::from_millis(100);

impl Controller {
    /// The screen for `now`, and when it changes next by itself.
    pub(super) fn screen(&self, now: Instant) -> (Screen, Option<Instant>) {
        let texts = self.texts;
        if self.voicemod != VoicemodStatus::Connected {
            let (message, waiting) = match self.voicemod {
                VoicemodStatus::Connecting => (texts.connecting, true),
                VoicemodStatus::KeyRejected | VoicemodStatus::MissingKey => (texts.update_app, false),
                _ => (texts.open_voicemod, false),
            };
            let screen = Screen::Voicemod {
                message: message.to_string(),
                waiting,
            };
            return (screen, None);
        }

        let counter = self.counter();
        if let Some(row) = self.knob.row.filter(|_| self.knob.pressed) {
            return (self.row_screen(row, counter), None);
        }
        if let Some(peek) = &self.peek {
            return (self.peek_screen(peek.pad, counter), None);
        }
        if let Some(menu) = &self.menu {
            let screen = Screen::Menu {
                heading: texts.soundboards.to_string(),
                items: self.pages.iter().map(|page| self.page_title(page)).collect(),
                selected: menu.selected,
                current: self.page,
            };
            return (screen, None);
        }

        let page = self.pages.get(self.page);
        let latest = self.playback.latest(now);
        let (name, name_until) = match (&self.running, &self.last, &latest) {
            // A macro shows by name while it runs
            (Some(running), ..) => (running.item.name.clone(), None),
            (None, Some((name, at)), _) => (name.clone(), Some(*at + LAST_SOUND_TIME)),
            // A sound still playing keeps its name above its bar
            (None, None, Some(latest)) => (self.sound_name(latest.id).to_string(), None),
            (None, None, None) if page.is_some() => (texts.ready.to_string(), None),
            (None, None, None) => (texts.no_sound.to_string(), None),
        };
        let (progress, changes) = self.progress(now);
        let (voice, voice_on) = self.voice_shown();
        let screen = Screen::Home {
            title: page.map_or("—".to_string(), |page| self.page_title(page)),
            counter,
            name,
            progress,
            voice,
            voice_on,
            switches: self.switches_shown(),
        };
        let progress_changes = changes.map(|after| now + after);
        (screen, name_until.into_iter().chain(progress_changes).min())
    }

    /// A sound's name, or the Community's for one outside the soundboards
    fn sound_name(&self, id: &str) -> &str {
        self.sounds.get(id).map_or(self.texts.community, |sound| &sound.name)
    }

    /// The page's number and the number of pages, such as "2/7".
    pub(super) fn counter(&self) -> String {
        if self.pages.is_empty() {
            String::new()
        } else {
            format!("{}/{}", self.page + 1, self.pages.len())
        }
    }

    /// The soundboard's name, or the app's own for the Favourites and Voices pages, in the language shown.
    pub(super) fn page_name<'a>(&self, page: &'a Page) -> &'a str {
        match page.soundboard_id.as_str() {
            FAVOURITES_ID => self.texts.favourites,
            VOICES_ID => self.texts.voices,
            _ => &page.name,
        }
    }

    pub(super) fn page_title(&self, page: &Page) -> String {
        let name = self.page_name(page);
        if page.parts > 1 {
            format!("{name} {}/{}", page.part, page.parts)
        } else {
            name.to_string()
        }
    }

    /// Below the name: the macro running, or what is left of the latest sound; and in how long that changes.
    fn progress(&self, now: Instant) -> (Option<Progress>, Option<Duration>) {
        if let Some(progress) = self.macro_progress() {
            let steps = Progress::Macro {
                step: progress.step,
                steps: progress.steps,
            };
            return (Some(steps), None);
        }
        let Some(latest) = self.playback.latest(now) else {
            return (None, None);
        };
        let texts = self.texts;
        // As its end was set: learned, else guessed
        let learned = self.playback.length(latest.id);
        let length = learned.unwrap_or(GUESSED_LENGTH);
        let share = |left: Duration| left.as_secs_f32() / length.max(left).as_secs_f32().max(f32::EPSILON);
        let endless = self
            .sounds
            .get(latest.id)
            .is_some_and(|sound| sound.looped || sound.playback_mode == PlaybackMode::PlayLoopOnPress);
        // A loop's round, where its length is known: what is left of it, the pauses left out
        let round_left = learned.filter(|length| !length.is_zero()).map(|length| {
            let into = latest.played.as_secs_f32() % length.as_secs_f32();
            1.0 - into / length.as_secs_f32()
        });
        let bar_step = (length / BAR_STEPS).max(FRAME);
        let (state, left, time, changes) = match (latest.state, latest.ends_at) {
            (State::Paused, _) if endless => (SoundState::Paused, round_left, String::new(), None),
            (State::Paused, _) => (
                SoundState::Paused,
                latest.left.map(share),
                latest.left.map(|left| texts.seconds(left)).unwrap_or_default(),
                None,
            ),
            (State::Playing, _) if endless => {
                let changes = round_left.map(|_| bar_step);
                (SoundState::Looping, round_left, String::new(), changes)
            }
            (State::Playing, Some(end)) => {
                let (shown, next_tenth) = countdown(end.saturating_duration_since(now));
                let changes = Some(next_tenth.min(bar_step));
                (SoundState::Playing, Some(share(shown)), texts.seconds(shown), changes)
            }
            (State::Playing, None) => (SoundState::Playing, None, String::new(), None),
        };
        (Some(Progress::Sound { state, left, time }), changes)
    }

    /// The voice loaded, by name when known, or that the voice changer is off; and whether it is on.
    fn voice_shown(&self) -> (Option<String>, bool) {
        if !self.switches.voice_changer {
            return (Some(self.texts.voice_off.to_string()), false);
        }
        let loaded = self
            .voice_loaded()
            .and_then(|id| self.voices.iter().find(|voice| voice.id == *id));
        (loaded.map(|voice| voice.name.clone()), true)
    }

    /// The switches away from how they usually are, from the right.
    fn switches_shown(&self) -> Vec<Switch> {
        let mic_silenced = self.switches.mic_muted
            || self
                .playback
                .active()
                .any(|id| self.sounds.get(id).is_some_and(|sound| sound.mute_voice));
        [
            (mic_silenced, Switch::MicOff),
            (self.switches.mute_for_me, Switch::MuteForMe),
            (self.switches.hear_myself, Switch::HearMe),
        ]
        .into_iter()
        .filter_map(|(on, switch)| on.then_some(switch))
        .collect()
    }

    fn peek_screen(&self, pad: u8, counter: String) -> Screen {
        let texts = self.texts;
        let pad_index = usize::from(pad);
        let (name, about) = match self.pad_at(pad) {
            Some(Pad::Action { name, .. }) => (name.as_str(), About::info(texts.action)),
            Some(Pad::Macro { name, .. }) => (name.as_str(), About::info(texts.macro_info)),
            _ => match (self.voice_at(pad), self.sound_at(pad)) {
                (Some(voice), _) if voice.enabled => (voice.name.as_str(), About::info(texts.voice)),
                (Some(voice), _) => (voice.name.as_str(), About::locked(texts.locked)),
                (None, Some(sound)) if !sound.enabled => (sound.name.as_str(), About::locked(texts.locked)),
                (None, Some(sound)) => {
                    let outcome = self.outcome_at(pad, sound);
                    let detail = if sound.looped && sound.playback_mode != PlaybackMode::PlayLoopOnPress {
                        texts.looped.to_string()
                    } else {
                        self.playback
                            .length(&sound.id)
                            .map(|length| texts.seconds(length))
                            .unwrap_or_default()
                    };
                    let note = if outcome.stops_all {
                        texts.stops_all
                    } else if sound.stop_other_sounds {
                        texts.stops_others
                    } else {
                        ""
                    };
                    let about = About {
                        info: texts.press_again(outcome.does, sound.playback_mode),
                        detail,
                        note,
                        locked: false,
                    };
                    (sound.name.as_str(), about)
                }
                (None, None) => (texts.empty, About::info("")),
            },
        };
        Screen::Peek {
            position: screens::position(pad_index / COLUMNS, pad_index % COLUMNS),
            counter,
            name: name.to_string(),
            info: about.info.to_string(),
            detail: about.detail,
            note: about.note.to_string(),
            locked: about.locked,
            pad: pad_index,
        }
    }

    fn row_screen(&self, row: usize, counter: String) -> Screen {
        let names = (0..COLUMNS)
            .map(|column| {
                let content = self.pad_at((row * COLUMNS + column) as u8);
                content.map(|content| content.name().to_string())
            })
            .collect();
        Screen::Row {
            heading: self.texts.row_heading(row, ROWS),
            counter,
            names,
        }
    }
}

/// What a peek tells about a pad, below its name
struct About {
    info: &'static str,
    detail: String,
    note: &'static str,
    locked: bool,
}

impl About {
    fn info(info: &'static str) -> Self {
        Self {
            info,
            detail: String::new(),
            note: "",
            locked: false,
        }
    }

    fn locked(info: &'static str) -> Self {
        Self {
            locked: true,
            ..Self::info(info)
        }
    }
}

/// What is left of a sound, rounded up to a tenth as shown, and in how long that changes.
fn countdown(left: Duration) -> (Duration, Duration) {
    let into = Duration::from_nanos((left.as_nanos() % TENTH.as_nanos()) as u64);
    if into.is_zero() {
        (left, TENTH)
    } else {
        (left - into + TENTH, into)
    }
}
