//! What the knob does: its gestures, recognised in `crate::knob`, then the action the user picked for each.

use super::{Controller, Level, MENU_TIME, Menu, Output, PEEK_TIME, ROWS, RowShown, VolumeTarget};
use crate::knob::{Gesture, KnobSettings, PressAction, TurnAction};
use crate::mixer::{self, Channel};
use crate::playback::State;
use crate::soundboard::{Action, Voice};
use crate::voicemod::{Request, VoicemodStatus};
use std::time::{Duration, Instant};

/// How long the row overview stays after a plain turn
const ROW_TIME: Duration = Duration::from_secs(4);
/// Voicemod gets at most one volume for each channel or sound in that time, the last one always
const VOLUME_EVERY: Duration = Duration::from_millis(100);

impl Controller {
    pub fn set_knob(&mut self, knob: KnobSettings) {
        self.knob_settings = knob;
        self.knob_waits();
    }

    /// A tap waits for a second one only while a double tap does something, and never in the open menu, where it picks.
    fn knob_waits(&mut self) {
        let settings = &self.knob_settings;
        let double_tap = settings.double_tap != PressAction::Nothing && self.menu.is_none();
        let long_press = settings.long_press != PressAction::Nothing;
        self.knob.set_waits(double_tap, long_press);
    }

    pub(super) fn knob_turned(&mut self, step: i8, now: Instant) {
        let gestures = self.knob.turned(step, now);
        self.gestures(gestures, now);
    }

    pub(super) fn knob_pressed(&mut self, pressed: bool, now: Instant) {
        if pressed == self.knob.is_down() {
            return;
        }
        self.knob_waits();
        let gestures = if pressed {
            self.knob_used = false;
            self.knob.pressed(now)
        } else {
            if self.knob_used {
                // The names stay a little after a peek or a row overview
                self.show_overlay(now);
            }
            // A row shown while held goes with the knob
            self.row = self.row.take().filter(|row| row.until.is_some());
            self.knob.released(now)
        };
        self.gestures(gestures, now);
    }

    /// A pad pressed while the knob is held: a peek, so the press is no tap.
    pub(super) fn knob_peeked(&mut self, now: Instant) {
        self.knob_used = true;
        let gestures = self.knob.pad_pressed();
        self.gestures(gestures, now);
    }

    /// A tap that waited for a second one, once it can no longer come.
    pub(super) fn update_knob(&mut self, now: Instant) {
        let gestures = self.knob.update(now);
        self.gestures(gestures, now);
        self.row = self.row.take().filter(|row| row.until.is_none_or(|until| until > now));
        self.level = self.level.take().filter(|level| level.until > now);
        self.mixer.keep_playing(|sound| self.playback.state(sound).is_some());
        self.send_waiting_volumes(now);
    }

    /// When the knob has something to do without any input.
    pub(super) fn knob_wake(&self) -> impl Iterator<Item = Instant> {
        let volumes = self
            .volumes_waiting
            .keys()
            .filter_map(|target| self.volumes_sent.get(target))
            .map(|sent| *sent + VOLUME_EVERY);
        let row = self.row.as_ref().and_then(|row| row.until);
        let level = self.level.as_ref().map(|level| level.until);
        volumes.chain(row).chain(level).chain(self.knob.next_update())
    }

    fn gestures(&mut self, gestures: Vec<Gesture>, now: Instant) {
        for gesture in gestures {
            self.outputs.push(Output::Gesture(gesture));
            let settings = &self.knob_settings;
            match gesture {
                // The open menu takes the turns and the taps, whatever the settings
                Gesture::Turn(step) => match &mut self.menu {
                    Some(menu) => {
                        let last = self.pages.len().saturating_sub(1) as isize;
                        menu.selected = (menu.selected as isize + isize::from(step)).clamp(0, last) as usize;
                        menu.until = now + MENU_TIME;
                    }
                    None => self.turn_action(settings.turn, step, false, now),
                },
                Gesture::HoldTurn(step) => {
                    self.knob_used = true;
                    self.peek = None;
                    self.turn_action(settings.hold_turn, step, true, now);
                }
                Gesture::Tap => match self.menu.take() {
                    Some(menu) => self.set_page(menu.selected.min(self.pages.len().saturating_sub(1)), now),
                    None => self.press_action(settings.tap.clone(), now),
                },
                Gesture::DoubleTap => self.press_action(settings.double_tap.clone(), now),
                Gesture::LongPress => self.press_action(settings.long_press.clone(), now),
            }
        }
    }

    /// `held`: the knob is down, so a row overview lasts until it is let go
    fn turn_action(&mut self, action: TurnAction, step: i8, held: bool, now: Instant) {
        let texts = self.texts;
        match action {
            TurnAction::Page => self.turn_page(step.into(), now),
            TurnAction::Rows => self.step_rows(step, held, now),
            TurnAction::SoundboardForOthers => {
                self.step_gain(Channel::SoundboardForOthers, texts.soundboard_others, step, now)
            }
            TurnAction::SoundboardForMe => self.step_gain(Channel::SoundboardForMe, texts.soundboard_you, step, now),
            TurnAction::Microphone => self.step_gain(Channel::Microphone, texts.microphone, step, now),
            TurnAction::PlayingSound => self.step_playing_sound(step, now),
            TurnAction::Voice => self.step_voice(step, now),
            TurnAction::Nothing => {}
        }
    }

    fn press_action(&mut self, action: PressAction, now: Instant) {
        match action {
            PressAction::Menu => {
                self.menu = Some(Menu {
                    selected: self.page,
                    until: now + MENU_TIME,
                })
            }
            PressAction::StopSounds => self.knob_act(Action::StopSounds, now),
            PressAction::MuteMic => self.knob_act(Action::MuteMic, now),
            PressAction::MuteForMe => self.knob_act(Action::MuteForMe, now),
            PressAction::VoiceChanger => self.knob_act(Action::VoiceChanger, now),
            PressAction::HearMyself => self.knob_act(Action::HearMyself, now),
            PressAction::Background => self.knob_act(Action::Background, now),
            PressAction::RandomVoice => self.knob_act(Action::RandomVoice, now),
            // One deleted since does nothing
            PressAction::Macro { macro_id } if self.voicemod == VoicemodStatus::Connected => {
                self.press_macro(&macro_id, now)
            }
            PressAction::Page { page } => self.act(Action::GoToPage, Some(&page), now),
            PressAction::ResetVolumes => self.reset_volumes(now),
            PressAction::Macro { .. } | PressAction::Nothing => {}
        }
    }

    /// As an action pad does: its name on the display, unless what it does shows by itself
    fn knob_act(&mut self, action: Action, now: Instant) {
        if self.voicemod != VoicemodStatus::Connected {
            return;
        }
        self.act(action, None, now);
        if !matches!(action, Action::StopSounds | Action::RandomVoice) {
            self.tell(self.texts.action_name(action), now);
        }
    }

    fn step_rows(&mut self, step: i8, held: bool, now: Instant) {
        let row = match self.row.as_ref().map(|row| row.row) {
            None if step > 0 => 0,
            None => ROWS - 1,
            Some(row) => (row as isize + isize::from(step)).rem_euclid(ROWS as isize) as usize,
        };
        self.row = Some(RowShown {
            row,
            until: (!held).then(|| now + ROW_TIME),
        });
        self.peek = None;
        self.level = None;
    }

    fn step_gain(&mut self, channel: Channel, label: &str, step: i8, now: Instant) {
        if !self.can_change_volumes(now) {
            return;
        }
        let Some(percent) = self.mixer.step_gain(channel, step) else {
            return self.tell(self.texts.not_available, now);
        };
        let gain = mixer::gain(percent);
        self.send_volume(VolumeTarget::Channel(channel), Request::SetGain { channel, gain }, now);
        self.show_level(label.to_string(), percent, channel.max_percent(), now);
    }

    /// The latest sound started that still plays
    fn step_playing_sound(&mut self, step: i8, now: Instant) {
        if !self.can_change_volumes(now) {
            return;
        }
        let playing = self
            .playback
            .latest(now)
            .filter(|latest| latest.state == State::Playing);
        let Some(id) = playing.map(|latest| latest.id.to_string()) else {
            return self.tell(self.texts.nothing_playing, now);
        };
        // Its saved volume not known yet: better nothing than a guess
        let Some(percent) = self.mixer.step_sound(&id, step) else {
            return self.tell(self.texts.not_available, now);
        };
        let volume = f32::from(percent) / 100.0;
        let label = self.sound_name(&id).to_string();
        let request = Request::SetSoundVolume { id: id.clone(), volume };
        self.send_volume(VolumeTarget::Sound(id), request, now);
        self.show_level(label, percent, 100, now);
    }

    /// The favourite voices in the licence, else all of them, in turn
    fn step_voice(&mut self, step: i8, now: Instant) {
        if self.voicemod != VoicemodStatus::Connected {
            return;
        }
        let licensed: Vec<&Voice> = self.voices.iter().filter(|voice| voice.enabled).collect();
        let favourites: Vec<&Voice> = licensed.iter().copied().filter(|voice| voice.favorited).collect();
        let voices = if favourites.is_empty() { licensed } else { favourites };
        if voices.is_empty() {
            return;
        }
        let count = voices.len() as isize;
        let at = self
            .voice_loaded()
            .and_then(|id| voices.iter().position(|voice| voice.id == *id));
        let next = match at {
            Some(at) => (at as isize + isize::from(step)).rem_euclid(count),
            None if step > 0 => 0,
            None => count - 1,
        };
        let voice = voices[next as usize].clone();
        self.load_voice(voice, now);
    }

    fn reset_volumes(&mut self, now: Instant) {
        if !self.can_change_volumes(now) {
            return;
        }
        let reset = self.mixer.reset();
        for (channel, gain) in reset.channels {
            self.send_volume(VolumeTarget::Channel(channel), Request::SetGain { channel, gain }, now);
        }
        for (id, volume) in reset.sounds {
            let request = Request::SetSoundVolume { id: id.clone(), volume };
            self.send_volume(VolumeTarget::Sound(id), request, now);
        }
        self.level = None;
        self.tell(self.texts.volumes_reset, now);
    }

    /// The volumes go through Voicemod's bridge: without it, the display says so
    fn can_change_volumes(&mut self, now: Instant) -> bool {
        if self.voicemod != VoicemodStatus::Connected {
            return false;
        }
        if !self.single_stop {
            self.tell(self.texts.not_available, now);
        }
        self.single_stop
    }

    fn tell(&mut self, text: &str, now: Instant) {
        self.level = None;
        self.last = Some((text.to_string(), now));
    }

    fn show_level(&mut self, label: String, percent: u16, max: u16, now: Instant) {
        self.row = None;
        self.level = Some(Level {
            label,
            percent,
            max,
            until: now + PEEK_TIME,
        });
    }

    fn send_volume(&mut self, target: VolumeTarget, request: Request, now: Instant) {
        let ready = self
            .volumes_sent
            .get(&target)
            .is_none_or(|sent| now >= *sent + VOLUME_EVERY);
        if ready {
            self.volumes_waiting.remove(&target);
            self.volumes_sent.insert(target, now);
            self.request(request);
        } else {
            self.volumes_waiting.insert(target, request);
        }
    }

    fn send_waiting_volumes(&mut self, now: Instant) {
        let due: Vec<VolumeTarget> = self
            .volumes_waiting
            .keys()
            .filter(|target| {
                self.volumes_sent
                    .get(*target)
                    .is_none_or(|sent| now >= *sent + VOLUME_EVERY)
            })
            .cloned()
            .collect();
        for target in due {
            if let Some(request) = self.volumes_waiting.remove(&target) {
                self.volumes_sent.insert(target, now);
                self.request(request);
            }
        }
    }

    /// The bridge came or went: the volumes are read again, nothing waits to be sent
    pub(super) fn volumes_forgotten(&mut self) {
        self.mixer.connected();
        self.volumes_sent.clear();
        self.volumes_waiting.clear();
    }
}
