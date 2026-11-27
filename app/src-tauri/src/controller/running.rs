//! The user's macros as they run: one at a time, as the controller's updates come. A macro on a time line starts each
//! sound at its time, stops it where it is cut and follows its volume; one from before the time line runs its steps one
//! after the other.

use super::{Controller, HeldSound, MIN_HOLD};
use crate::macros::{Again, Clip, Macro, Step, Switch, SwitchState, Timeline};
use crate::mixer::Channel;
use crate::press_again::Command;
use crate::soundboard::{PlaybackMode, Voice};
use crate::voicemod::{Request, VoicemodStatus};
use serde::Serialize;
use std::collections::HashSet;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// A sound waited for that never ends, as a loop, holds the macro this long at most
const MAX_SOUND_WAIT: Duration = Duration::from_secs(30);
/// A round of steps that take no time is not run again sooner
const MIN_ROUND: Duration = Duration::from_millis(500);
/// A sound's volume along its curve goes to Voicemod at most that often
const LEVEL_EVERY: Duration = Duration::from_millis(100);
/// A sound of known length may end that much later than its length says, as the log tells it a moment late
const END_SLACK: Duration = Duration::from_secs(2);

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
    /// When the round started, for the window, which shows where the time line is
    round_wall: SystemTime,
    /// On a time line: what is left to do in this round
    played: Option<Played>,
    /// The sounds it started, to stop with it
    started: HashSet<String>,
}

/// A round of a macro on its time line
struct Played {
    /// What happens when, from the round's start, in order
    events: Vec<(Duration, Event)>,
    next: usize,
    /// The clips and actions started, for the display
    done: usize,
    /// The clips with a volume of their own that play
    shaped: Vec<Shaped>,
    /// When the clip that waits for its sound's options was due: the time line goes on from there once it plays
    held: Option<Instant>,
}

#[derive(Clone)]
enum Event {
    Start(Clip),
    Cut(String),
    Action(Step),
}

struct Shaped {
    clip: Clip,
    started: Instant,
    /// The level last sent, and when
    sent: (u8, Instant),
}

impl Played {
    fn of(timeline: &Timeline) -> Self {
        let millis = |at: u32| Duration::from_millis(at.into());
        let mut events: Vec<(Duration, Event)> = Vec::new();
        for clip in &timeline.clips {
            events.push((millis(clip.at), Event::Start(clip.clone())));
            if let Some(cut) = clip.cut {
                events.push((millis(clip.at.saturating_add(cut)), Event::Cut(clip.id.clone())));
            }
        }
        for action in &timeline.actions {
            events.push((millis(action.at), Event::Action(action.step.clone())));
        }
        events.sort_by_key(|(at, event)| (*at, rank(event)));
        Self {
            events,
            next: 0,
            done: 0,
            shaped: Vec::new(),
            held: None,
        }
    }

    /// A cut found on the way, among those still to come
    fn add_cut(&mut self, at: Duration, id: String) {
        let ahead = self.events[self.next..].partition_point(|(other, event)| (*other, rank(event)) <= (at, 0));
        self.events.insert(self.next + ahead, (at, Event::Cut(id)));
    }
}

/// At the same time, a cut and an action come before a start: a stop then a sound plays the sound
fn rank(event: &Event) -> u8 {
    match event {
        Event::Cut(_) => 0,
        Event::Action(_) => 1,
        Event::Start(_) => 2,
    }
}

/// Where the macro that runs is, for the window
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MacroProgress {
    pub id: String,
    /// The step running, from 1
    pub step: usize,
    pub steps: usize,
    /// When the round under way started, in milliseconds since 1970: the window draws where the time line is
    pub round_started: u64,
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

    /// The editor's Stop, whatever a press on its pads does: its sounds stop with it.
    pub fn stop_macro(&mut self, now: Instant) {
        self.silence_macro(now);
        self.finish_macro();
    }

    /// Stops the sounds the macro running started that still play, as a stop asked by the user does: one by one
    /// through the bridge, else every sound
    fn silence_macro(&mut self, now: Instant) {
        let Some(running) = self.running.as_mut() else {
            return;
        };
        let playing: Vec<String> = std::mem::take(&mut running.started)
            .into_iter()
            .filter(|id| self.playback.state(id).is_some())
            .collect();
        if playing.is_empty() {
            return;
        }
        if !self.single_stop {
            return self.stop_every_sound(now);
        }
        for id in playing {
            self.request(Request::StopSound(id.clone()));
            self.playback.stop(&id, now);
        }
    }

    pub fn macro_progress(&self) -> Option<MacroProgress> {
        let running = self.running.as_ref()?;
        let since = running.round_wall.duration_since(UNIX_EPOCH).unwrap_or_default();
        let (step, steps) = match (&running.played, &running.item.timeline) {
            (Some(played), Some(timeline)) => (played.done, timeline.clips.len() + timeline.actions.len()),
            _ => (running.step, running.item.steps.len()),
        };
        Some(MacroProgress {
            id: running.item.id.clone(),
            // Between two rounds, the first step is next
            step: step.max(1),
            steps,
            round_started: since.as_millis() as u64,
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
                self.silence_macro(now);
                self.finish_macro();
                return;
            }
            Some(Again::Restart) => {
                self.silence_macro(now);
                if let Some(running) = &mut self.running {
                    running.step = 0;
                    running.rounds = 0;
                    running.round_started = now;
                    running.round_wall = SystemTime::now();
                    running.next_at = now;
                    running.waiting = None;
                    running.played = running.item.timeline.as_ref().map(Played::of);
                }
            }
            None => {
                // One deleted since does nothing, the one running goes on
                let Some(item) = self.macros.iter().find(|item| item.id == id).cloned() else {
                    return;
                };
                // Another one stops, its sounds with it
                self.silence_macro(now);
                self.finish_macro();
                self.override_sounds(&item, now);
                let played = item.timeline.as_ref().map(Played::of);
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
                    round_wall: SystemTime::now(),
                    played,
                    started: HashSet::new(),
                });
            }
        }
        self.advance_macro(now);
    }

    /// Runs the steps due by `now`.
    pub(super) fn advance_macro(&mut self, now: Instant) {
        if self.running.as_ref().is_some_and(|running| running.played.is_some()) {
            return self.advance_timeline(now);
        }
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
            running.round_wall = SystemTime::now() + (running.next_at - now);
        }
    }

    /// Runs what is due on the time line by `now`, follows the volumes, and ends the round once its sounds end.
    fn advance_timeline(&mut self, now: Instant) {
        loop {
            let Some(running) = self.running.as_ref() else {
                return;
            };
            let start = running.round_started;
            let Some(played) = running.played.as_ref() else {
                return;
            };
            let due = played
                .events
                .get(played.next)
                .filter(|(at, _)| start + *at <= now)
                .map(|(at, event)| (start + *at, event.clone()));
            let Some((due_at, event)) = due else {
                break;
            };
            // A sound that would stop or mute the others playing waits for its options to change, a moment at most, as
            // does one a sound playing would mute
            let holds = matches!(&event, Event::Start(clip)
                if (self.override_pending(&clip.id, now) && self.others_playing(&clip.id))
                    || self.muted_meanwhile(&clip.id, |other| self.mute_pending(other, now)));
            let Some(running) = self.running.as_mut() else {
                return;
            };
            let Some(played) = running.played.as_mut() else {
                return;
            };
            if holds {
                played.held.get_or_insert(due_at);
                break;
            }
            // The rest of the time line keeps its distance from the clip that waited
            if let Some(since) = played.held.take() {
                let waited = now.saturating_duration_since(since);
                running.round_started += waited;
                running.round_wall += waited;
                continue;
            }
            played.next += 1;
            if !matches!(event, Event::Cut(_)) {
                played.done += 1;
            }
            self.run_event(event, now);
        }
        self.follow_levels(now);

        let Some(running) = self.running.as_ref() else {
            return;
        };
        let Some(played) = running.played.as_ref() else {
            return;
        };
        if played.next < played.events.len() {
            return;
        }
        let deadline = self.round_deadline(running.round_started, played);
        let sounds: Vec<String> = played
            .events
            .iter()
            .filter_map(|(_, event)| match event {
                Event::Start(clip) => Some(clip.id.clone()),
                _ => None,
            })
            .collect();
        let playing = sounds.iter().any(|id| self.playback.state(id).is_some());
        if playing && now < deadline {
            return;
        }
        let Some(running) = self.running.as_mut() else {
            return;
        };
        running.rounds = running.rounds.saturating_add(1);
        let repeat = running.item.repeat;
        let empty = running.played.as_ref().is_some_and(|played| played.events.is_empty());
        if empty || (repeat != 0 && running.rounds >= repeat) {
            self.finish_macro();
            return;
        }
        running.round_started = (running.round_started + MIN_ROUND).max(now);
        running.round_wall = SystemTime::now() + (running.round_started - now);
        running.played = running.item.timeline.as_ref().map(Played::of);
        self.advance_timeline(now);
    }

    fn run_event(&mut self, event: Event, now: Instant) {
        match event {
            Event::Start(clip) => {
                let Some(sound) = self.sounds.get(&clip.id).filter(|sound| sound.enabled).cloned() else {
                    return;
                };
                self.command(Command::KeyDown, &sound, now);
                self.sound_started();
                // A sound that loops plays once, as the time line draws it, should its loop not be turned off in time
                let once = (clip.cut.is_none() && (sound.looped || self.overridden_loop(&clip.id)))
                    .then(|| self.sound_length(&clip.id))
                    .flatten()
                    .filter(|_| sound.playback_mode != PlaybackMode::PlayLoopOnPress);
                if let Some(running) = self.running.as_mut() {
                    running.started.insert(sound.id.clone());
                    if let (Some(length), Some(played)) = (once, running.played.as_mut()) {
                        let end = Duration::from_millis(clip.at.into()) + length;
                        played.add_cut(end, clip.id.clone());
                    }
                }
                if sound.playback_mode == PlaybackMode::PlayLoopOnPress {
                    let length = clip.cut.map(|cut| Duration::from_millis(cut.into()));
                    let hold = length.or(self.sound_length(&clip.id)).unwrap_or(MIN_HOLD).max(MIN_HOLD);
                    let held = HeldSound {
                        sound,
                        since: now,
                        release: Command::KeyUp,
                    };
                    self.releases.push((now + hold, held));
                }
                // Its curve needs the bridge, as the knob's volumes do
                let Some(level) = clip.level_at(0).filter(|_| self.single_stop) else {
                    return;
                };
                self.send_level(&clip.id, level);
                if let Some(played) = self.running.as_mut().and_then(|running| running.played.as_mut()) {
                    played.shaped.retain(|shaped| shaped.clip.id != clip.id);
                    played.shaped.push(Shaped {
                        clip,
                        started: now,
                        sent: (level, now),
                    });
                }
            }
            // Without the bridge it would stop every sound: the sound plays to its end instead
            Event::Cut(id) => {
                if self.single_stop && self.playback.state(&id).is_some() {
                    self.request(Request::StopSound(id.clone()));
                    self.playback.stop(&id, now);
                }
            }
            Event::Action(step) => self.run_step(step, now),
        }
    }

    /// Each sound with a curve of its own at its level now, at most every 100 ms
    fn follow_levels(&mut self, now: Instant) {
        let Some(played) = self.running.as_mut().and_then(|running| running.played.as_mut()) else {
            return;
        };
        let mut shaped = std::mem::take(&mut played.shaped);
        shaped.retain(|shaped| self.playback.state(&shaped.clip.id).is_some());
        let mut finished = Vec::new();
        for (index, shaped) in shaped.iter_mut().enumerate() {
            if now < shaped.sent.1 + LEVEL_EVERY {
                continue;
            }
            let at = now
                .saturating_duration_since(shaped.started)
                .as_millis()
                .min(u32::MAX.into()) as u32;
            let Some(level) = shaped.clip.level_at(at) else {
                continue;
            };
            if level != shaped.sent.0 {
                self.send_level(&shaped.clip.id, level);
            }
            // Looked at again in 100 ms, even on a flat stretch
            shaped.sent = (level, now);
            // Past its last level, it stays there: nothing more to follow
            if shaped.clip.volume.last().is_some_and(|last| at >= last.at) {
                finished.push(index);
            }
        }
        for index in finished.into_iter().rev() {
            shaped.remove(index);
        }
        if let Some(played) = self.running.as_mut().and_then(|running| running.played.as_mut()) {
            played.shaped = shaped;
        }
    }

    /// When a round ends at the latest though its sounds still play: once every sound of known length has played
    /// through, or 30 s after its last event for one that loops or whose length is not known
    fn round_deadline(&self, started: Instant, played: &Played) -> Instant {
        let last = played.events.last().map_or(Duration::ZERO, |(at, _)| *at);
        let ends = played.events.iter().filter_map(|(at, event)| match event {
            Event::Start(clip) => {
                let cut = clip.cut.map(|cut| Duration::from_millis(cut.into()));
                Some(*at + cut.or(self.sound_length(&clip.id))?)
            }
            _ => None,
        });
        let known = ends.max().map_or(Duration::ZERO, |end| end + END_SLACK);
        started + (last + MAX_SOUND_WAIT).max(known)
    }

    /// Another sound the macro running started still plays: one that stops or mutes the others would cut it
    fn others_playing(&self, id: &str) -> bool {
        self.quiets_others(id)
            && self.running.as_ref().is_some_and(|running| {
                running
                    .started
                    .iter()
                    .any(|other| other != id && self.playback.state(other).is_some())
            })
    }

    /// Another sound the macro running started still plays that would mute this one, as `muting` tells
    fn muted_meanwhile(&self, id: &str, muting: impl Fn(&str) -> bool) -> bool {
        self.running.as_ref().is_some_and(|running| {
            running
                .started
                .iter()
                .any(|other| other != id && muting(other) && self.playback.state(other).is_some())
        })
    }

    /// From its file, as Voicemod tells it, else as the log taught it
    fn sound_length(&self, id: &str) -> Option<Duration> {
        self.file_lengths.get(id).copied().or(self.playback.length(id))
    }

    fn send_level(&mut self, id: &str, percent: u8) {
        let volume = f32::from(percent) / 100.0;
        self.request(Request::SetSoundVolume {
            id: id.to_string(),
            volume,
        });
        self.mixer.play_at(id, volume);
    }

    /// When the macro running has its next step due.
    pub(super) fn macro_wake(&self) -> Option<Instant> {
        let running = self.running.as_ref()?;
        if let Some(played) = &running.played {
            // A clip waiting for its options wakes with them, or when it stops waiting
            let next = played
                .events
                .get(played.next)
                .filter(|(_, event)| {
                    !matches!(event, Event::Start(clip)
                        if (self.override_asked(&clip.id) && self.others_playing(&clip.id))
                            || self.muted_meanwhile(&clip.id, |other| self.mute_asked(other)))
                })
                .map(|(at, _)| running.round_started + *at);
            let level = played.shaped.iter().map(|shaped| shaped.sent.1 + LEVEL_EVERY).min();
            // The round's end, should its last sound never end
            let end = self.round_deadline(running.round_started, played);
            return [next, level, Some(end)].into_iter().flatten().min();
        }
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
                if let Some(running) = self.running.as_mut() {
                    running.started.insert(sound.id.clone());
                }
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
        if switch == Switch::MuteForMe {
            self.mixer.set_on(Channel::SoundboardForMe, !on);
        }
        self.request(request);
    }
}
