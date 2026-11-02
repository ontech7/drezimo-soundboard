//! What the display shows, as in the simulator.

use super::{COLUMNS, Controller, ROWS};
use crate::display::screens::Screen;
use crate::playback::State;
use crate::soundboard::{FAVOURITES_ID, Pad, Page, PlaybackMode, VOICES_ID};
use crate::voicemod::VoicemodStatus;
use std::time::{Duration, Instant};

/// How long the last sound's name stays on the display
pub(super) const LAST_SOUND_TIME: Duration = Duration::from_secs(8);
/// How often a countdown changes
const COUNTDOWN_STEP: Duration = Duration::from_millis(100);

impl Controller {
    /// The screen for `now`, and when it changes next by itself.
    pub(super) fn screen(&self, now: Instant) -> (Screen, Option<Instant>) {
        let texts = self.texts;
        if self.voicemod != VoicemodStatus::Connected {
            let message = match self.voicemod {
                VoicemodStatus::Connecting => texts.connecting,
                VoicemodStatus::KeyRejected | VoicemodStatus::MissingKey => texts.update_app,
                _ => texts.unreachable,
            };
            let screen = Screen::Voicemod {
                message: message.to_string(),
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
            };
            return (screen, None);
        }

        let page = self.pages.get(self.page);
        let (name, name_until) = match &self.last {
            Some((name, at)) => (name.clone(), Some(*at + LAST_SOUND_TIME)),
            None if page.is_some() => (texts.ready.to_string(), None),
            None => (texts.no_sound.to_string(), None),
        };
        let (right, counting) = self.status(now);
        let screen = Screen::Home {
            title: page.map_or("—".to_string(), |page| self.page_title(page)),
            counter,
            name,
            left: "LIVE".to_string(),
            right,
        };
        let countdown = counting.then(|| now + COUNTDOWN_STEP);
        (screen, name_until.into_iter().chain(countdown).min())
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

    /// The bottom right of the home screen: the latest sound's state or, when it has none to show, Voicemod's switches.
    /// True while it counts down.
    fn status(&self, now: Instant) -> (String, bool) {
        let texts = self.texts;
        let mut parts = Vec::new();
        let mic_silenced = self.switches.mic_muted
            || self
                .playback
                .active()
                .any(|id| self.sounds.get(id).is_some_and(|sound| sound.mute_voice));
        if mic_silenced {
            parts.push(texts.mic_off.to_string());
        }
        let sound_state = self.playback.latest().and_then(|(id, state, ends_at)| {
            let endless = self
                .sounds
                .get(id)
                .is_some_and(|sound| sound.looped || sound.playback_mode == PlaybackMode::PlayLoopOnPress);
            if state == State::Paused {
                Some((texts.paused.to_string(), false))
            } else if endless {
                Some((texts.looped.to_string(), false))
            } else {
                ends_at.map(|end| (seconds(end.saturating_duration_since(now)), true))
            }
        });
        let shown = sound_state.or_else(|| Some((self.switch_status()?.to_string(), false)));
        let counting = shown.as_ref().is_some_and(|(_, counting)| *counting);
        parts.extend(shown.map(|(text, _)| text));
        (parts.join(" "), counting)
    }

    /// The first switch away from how it usually is, if any.
    fn switch_status(&self) -> Option<&'static str> {
        let texts = self.texts;
        if self.switches.mute_for_me {
            Some(texts.mute_me)
        } else if !self.switches.voice_changer {
            Some(texts.voice_off)
        } else if self.switches.hear_myself {
            Some(texts.hear_me)
        } else {
            None
        }
    }

    fn peek_screen(&self, pad: u8, counter: String) -> Screen {
        let texts = self.texts;
        let pad_index = usize::from(pad);
        let position = format!("R{} C{}", pad_index / COLUMNS + 1, pad_index % COLUMNS + 1);
        if let Some(Pad::Action { name, .. }) = self.pad_at(pad) {
            return Screen::Peek {
                position,
                counter,
                name: name.clone(),
                info: texts.action.to_string(),
                note: String::new(),
            };
        }
        if let Some(voice) = self.voice_at(pad) {
            return Screen::Peek {
                position,
                counter,
                name: voice.name.clone(),
                info: if voice.enabled { texts.voice } else { texts.locked }.to_string(),
                note: String::new(),
            };
        }
        let Some(sound) = self.sound_at(pad) else {
            return Screen::Peek {
                position,
                counter,
                name: texts.empty.to_string(),
                info: String::new(),
                note: String::new(),
            };
        };
        let info = if sound.enabled {
            let detail = if sound.looped && sound.playback_mode != PlaybackMode::PlayLoopOnPress {
                texts.looped.to_string()
            } else {
                self.playback.length(&sound.id).map(seconds).unwrap_or_default()
            };
            [texts.mode(sound.playback_mode), &detail]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            texts.locked.to_string()
        };
        let stops_others = sound.stop_other_sounds && sound.enabled;
        Screen::Peek {
            position,
            counter,
            name: sound.name.clone(),
            info,
            note: if stops_others { texts.stops_others } else { "" }.to_string(),
        }
    }

    fn row_screen(&self, row: usize, counter: String) -> Screen {
        let lines = (0..COLUMNS)
            .map(|column| {
                let content = self.pad_at((row * COLUMNS + column) as u8);
                format!("{} {}", column + 1, content.map_or("-", |content| content.name()))
            })
            .collect();
        Screen::Row {
            heading: format!("{} {}/{ROWS}", self.texts.row, row + 1),
            counter,
            lines,
        }
    }
}

/// A duration as the simulator shows it, such as "1.5s".
fn seconds(duration: Duration) -> String {
    format!("{:.1}s", duration.as_secs_f64())
}
