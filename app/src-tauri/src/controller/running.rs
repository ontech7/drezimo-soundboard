//! The user's macros as they run: one at a time, step after step, as the controller's updates come.

use super::{Controller, HeldSound, MIN_HOLD};
use crate::macros::{Again, Macro, Step, Switch, SwitchState};
use crate::press_again::Command;
use crate::soundboard::{PlaybackMode, Voice};
use crate::voicemod::{Request, VoicemodStatus};
use serde::Serialize;
use std::time::{Duration, Instant};

/// A sound waited for that never ends, as a loop, holds the macro this long at most
const MAX_SOUND_WAIT: Duration = Duration::from_secs(30);
/// A round of steps that take no time is not run again sooner
const MIN_ROUND: Duration = Duration::from_millis(500);

pub(super) struct Running {
    pub(super) item: Macro,
    /// The next step to run
    step: usize,
    rounds: u8,
    round_started: Instant,
    next_at: Instant,
    /// The sound the macro waits for, and until when at most
    waiting: Option<(String, Instant)>,
    /// The voice loaded when it started, and whether a step changed it
    voice_before: Option<String>,
    voice_changed: bool,
    /// The switches a step set, as they were before
    switches_before: Vec<(Switch, bool)>,
}

/// Where the macro that runs is, for the window
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MacroProgress {
    pub id: String,
    /// The step running, from 1
    pub step: usize,
    pub steps: usize,
}

impl Controller {
    /// The user's macros, as the editor saved them. One running goes on as it started, unless it was deleted.
    pub fn set_macros(&mut self, macros: Vec<Macro>) {
        let deleted = self
            .running
            .as_ref()
            .is_some_and(|running| !macros.iter().any(|item| item.id == running.item.id));
        if deleted {
            self.finish_macro();
        }
        self.macros = macros;
        if self.has_lists() {
            self.rebuild_pages(false);
        }
    }

    /// The editor's Try, as a press on a pad of the macro.
    pub fn try_macro(&mut self, id: &str, now: Instant) {
        if self.voicemod == VoicemodStatus::Connected {
            self.press_macro(id, now);
        }
    }

    /// The editor's Stop, whatever a press on its pads does.
    pub fn stop_macro(&mut self) {
        self.finish_macro();
    }

    pub fn macro_progress(&self) -> Option<MacroProgress> {
        let running = self.running.as_ref()?;
        Some(MacroProgress {
            id: running.item.id.clone(),
            // Between two rounds, the first step is next
            step: running.step.max(1),
            steps: running.item.steps.len(),
        })
    }

    /// Starts the macro, or stops or restarts it while it runs. Another one running stops first.
    pub(super) fn press_macro(&mut self, id: &str, now: Instant) {
        let again = self
            .running
            .as_ref()
            .filter(|running| running.item.id == id)
            .map(|running| running.item.again);
        match again {
            Some(Again::Stop) => {
                self.finish_macro();
                return;
            }
            Some(Again::Restart) => {
                if let Some(running) = &mut self.running {
                    running.step = 0;
                    running.rounds = 0;
                    running.round_started = now;
                    running.next_at = now;
                    running.waiting = None;
                }
            }
            None => {
                // One deleted since does nothing, the one running goes on
                let Some(item) = self.macros.iter().find(|item| item.id == id).cloned() else {
                    return;
                };
                self.finish_macro();
                self.running = Some(Running {
                    item,
                    step: 0,
                    rounds: 0,
                    round_started: now,
                    next_at: now,
                    waiting: None,
                    voice_before: self.voice_loaded().cloned(),
                    voice_changed: false,
                    switches_before: Vec::new(),
                });
            }
        }
        self.advance_macro(now);
    }

    /// Runs the steps due by `now`.
    pub(super) fn advance_macro(&mut self, now: Instant) {
        loop {
            let Some(running) = self.running.as_ref() else {
                return;
            };
            let waiting = running
                .waiting
                .as_ref()
                .is_some_and(|(id, until)| now < *until && self.playback.state(id).is_some());
            if waiting || now < running.next_at {
                return;
            }
            let Some(running) = self.running.as_mut() else {
                return;
            };
            running.waiting = None;
            if let Some(step) = running.item.steps.get(running.step).cloned() {
                running.step += 1;
                self.run_step(step, now);
                continue;
            }
            running.rounds = running.rounds.saturating_add(1);
            let repeat = running.item.repeat;
            if running.item.steps.is_empty() || (repeat != 0 && running.rounds >= repeat) {
                self.finish_macro();
                return;
            }
            running.step = 0;
            running.next_at = (running.round_started + MIN_ROUND).max(now);
            running.round_started = running.next_at;
        }
    }

    /// When the macro running has its next step due.
    pub(super) fn macro_wake(&self) -> Option<Instant> {
        let running = self.running.as_ref()?;
        Some(running.waiting.as_ref().map_or(running.next_at, |(_, until)| *until))
    }

    /// Ends the macro running, putting back what it changed if it says so.
    pub(super) fn finish_macro(&mut self) {
        let Some(running) = self.running.take() else {
            return;
        };
        if !running.item.restore {
            return;
        }
        let voice = running.voice_before.filter(|_| running.voice_changed);
        if let Some(voice) = voice.filter(|voice| self.voice_loaded() != Some(voice)) {
            // Put back quietly: the display keeps what it shows
            self.ask_voice(voice);
        }
        for (switch, before) in running.switches_before {
            self.set_switch(switch, before);
        }
    }

    fn run_step(&mut self, step: Step, now: Instant) {
        match step {
            Step::Sound { id, wait_end, .. } => {
                let Some(sound) = self.sounds.get(&id).filter(|sound| sound.enabled).cloned() else {
                    return;
                };
                self.command(Command::KeyDown, &sound, now);
                self.sound_started();
                if sound.playback_mode == PlaybackMode::PlayLoopOnPress {
                    // Held for its length, once known, else as a tap
                    let hold = self.playback.length(&id).unwrap_or(MIN_HOLD).max(MIN_HOLD);
                    let held = HeldSound {
                        sound,
                        since: now,
                        release: Command::KeyUp,
                    };
                    self.releases.push((now + hold, held));
                }
                if let Some(running) = self.running.as_mut().filter(|_| wait_end) {
                    running.waiting = Some((id, now + MAX_SOUND_WAIT));
                }
            }
            Step::Wait { millis } => {
                if let Some(running) = &mut self.running {
                    running.next_at = now + Duration::from_millis(millis.into());
                }
            }
            Step::Voice { id, .. } => {
                let voice = self
                    .voices
                    .iter()
                    .find(|voice| voice.id == id && voice.enabled)
                    .cloned();
                self.macro_voice(voice, now);
            }
            Step::RandomVoice => {
                let voice = self.random_other_voice(now);
                self.macro_voice(voice, now);
            }
            Step::PreviousVoice => {
                let before = self.running.as_ref().and_then(|running| running.voice_before.clone());
                let voice = before.and_then(|id| self.voices.iter().find(|voice| voice.id == id).cloned());
                self.macro_voice(voice, now);
            }
            Step::Switch { switch, state } => {
                let current = self.switch_on(switch);
                if let Some(running) = &mut self.running
                    && !running.switches_before.iter().any(|(set, _)| *set == switch)
                {
                    running.switches_before.push((switch, current));
                }
                let wanted = match state {
                    SwitchState::On => true,
                    SwitchState::Off => false,
                    SwitchState::Toggle => !current,
                };
                self.set_switch(switch, wanted);
            }
            Step::StopSounds => self.stop_sounds(now),
            Step::GoToPage { page } => {
                if let Some(target) = self.pages.iter().position(|shown| shown.soundboard_id == page) {
                    self.set_page(target, now);
                }
            }
        }
    }

    fn macro_voice(&mut self, voice: Option<Voice>, now: Instant) {
        let Some(voice) = voice.filter(|voice| self.voice_loaded() != Some(&voice.id)) else {
            return;
        };
        self.load_voice(voice, now);
        if let Some(running) = &mut self.running {
            running.voice_changed = true;
        }
    }

    fn switch_on(&self, switch: Switch) -> bool {
        let switches = &self.switches;
        match switch {
            Switch::MuteMic => switches.mic_muted,
            Switch::VoiceChanger => switches.voice_changer,
            Switch::HearMyself => switches.hear_myself,
            Switch::MuteForMe => switches.mute_for_me,
            Switch::Background => switches.background,
        }
    }

    /// Toggles the switch if it is not as wanted. Taken as done at once: a later step sees it so before Voicemod
    /// confirms it.
    fn set_switch(&mut self, switch: Switch, on: bool) {
        if self.switch_on(switch) == on {
            return;
        }
        let switches = &mut self.switches;
        let (request, value) = match switch {
            Switch::MuteMic => (Request::ToggleMuteMic, &mut switches.mic_muted),
            Switch::VoiceChanger => (Request::ToggleVoiceChanger, &mut switches.voice_changer),
            Switch::HearMyself => (Request::ToggleHearMyself, &mut switches.hear_myself),
            Switch::MuteForMe => (Request::ToggleMuteForMe, &mut switches.mute_for_me),
            Switch::Background => (Request::ToggleBackground, &mut switches.background),
        };
        *value = on;
        self.request(request);
    }
}
