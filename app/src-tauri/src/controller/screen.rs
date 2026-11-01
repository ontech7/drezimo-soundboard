//! What the display shows, as in the simulator.

use super::{COLUMNS, Controller, ROWS};
use crate::display::screens::Screen;
use crate::playback::State;
use crate::soundboard::{FAVOURITES_ID, Page, PlaybackMode};
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

    pub(super) fn page_title(&self, page: &Page) -> String {
        let name = if page.soundboard_id == FAVOURITES_ID {
            self.texts.favourites
        } else {
            &page.name
        };
        if page.parts > 1 {
            format!("{name} {}/{}", page.part, page.parts)
        } else {
            name.to_string()
        }
    }

    /// The bottom right of the home screen: the latest sound's state, or Voicemod's switches. True while it counts
    /// down.
    fn status(&self, now: Instant) -> (String, bool) {
        let texts = self.texts;
        let mut parts = Vec::new();
        let mic_silenced = self
            .playback
            .active()
            .any(|id| self.sounds.get(id).is_some_and(|sound| sound.mute_voice));
        if mic_silenced {
            parts.push(texts.mic_off.to_string());
        }
        let mut counting = false;
        if let Some((id, state, ends_at)) = self.playback.latest() {
            let endless = self
                .sounds
                .get(id)
                .is_some_and(|sound| sound.looped || sound.playback_mode == PlaybackMode::PlayLoopOnPress);
            if state == State::Paused {
                parts.push(texts.paused.to_string());
            } else if endless {
                parts.push(texts.looped.to_string());
            } else if let Some(end) = ends_at {
                parts.push(seconds(end.saturating_duration_since(now)));
                counting = true;
            }
        } else if self.switches.mute_for_me {
            parts.push(texts.mute_me.to_string());
        } else if !self.switches.voice_changer {
            parts.push(texts.voice_off.to_string());
        } else if self.switches.hear_myself {
            parts.push(texts.hear_me.to_string());
        }
        (parts.join(" "), counting)
    }

    fn peek_screen(&self, pad: u8, counter: String) -> Screen {
        let texts = self.texts;
        let pad_index = usize::from(pad);
        let position = format!("R{} C{}", pad_index / COLUMNS + 1, pad_index % COLUMNS + 1);
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
                let sound = self.sound_at((row * COLUMNS + column) as u8);
                format!("{} {}", column + 1, sound.map_or("-", |sound| &sound.name))
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
