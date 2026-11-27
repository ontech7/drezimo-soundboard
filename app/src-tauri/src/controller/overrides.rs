//! A macro's sounds play as its time line draws them, over each other and once each: for its run, the options Voicemod
//! keeps for each of them (stop the other sounds when it starts, mute them while it plays, loop) are turned off through
//! the bridge, then put back once the sound is quiet and no macro running uses it (docs/project.md, section 8). They are
//! kept in a file until Voicemod confirms them put back, so an app closed halfway puts them back at its next start.

use super::{Controller, Output};
use crate::macros::Macro;
use crate::voicemod::{Request, VoicemodStatus};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

/// A clip that would stop or mute the macro's other sounds waits this long at most for its options to change, then
/// plays all the same. Voicemod answers in 0.1–0.2 s, the first change after a while idle in 1.2–1.4 s.
pub(super) const OVERRIDE_WAIT: Duration = Duration::from_millis(1500);
/// Options put back that Voicemod has not confirmed are sent again after this long
const RESTORE_RETRY: Duration = Duration::from_secs(5);
/// Sent this many times without Voicemod confirming them, they are given up
const RESTORE_TRIES: u8 = 3;
/// Voicemod's soundboards asked for before options were put back may come after: those put back hold over them so long
const RESTORED_HOLD: Duration = Duration::from_secs(30);

/// What Voicemod does with a sound, of what a macro turns off
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnOptions {
    pub stops_others: bool,
    pub mutes_others: bool,
    pub loops: bool,
}

impl OwnOptions {
    fn any(self) -> bool {
        self.stops_others || self.mutes_others || self.loops
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum State {
    /// Turned off, since then, until Voicemod answers
    Asked(Instant),
    Applied,
    /// Put back, since then, until Voicemod confirms it
    Restoring(Instant),
}

pub(super) struct Override {
    /// As Voicemod had them, to put back
    own: OwnOptions,
    state: State,
    /// The changes Voicemod has not answered yet, in order: whether each puts them back, and when it went
    sent: VecDeque<(bool, Instant)>,
    /// How many times they were put back without Voicemod confirming it
    tries: u8,
}

impl Override {
    fn new(own: OwnOptions, state: State) -> Self {
        Self {
            own,
            state,
            sent: VecDeque::new(),
            tries: 0,
        }
    }
}

impl Controller {
    /// Turns off what would stop a macro's sounds playing over each other, for those that need it
    pub(super) fn override_sounds(&mut self, item: &Macro, now: Instant) {
        if !self.single_stop || self.voicemod != VoicemodStatus::Connected {
            return;
        }
        let Some(timeline) = &item.timeline else {
            return;
        };
        let ids: HashSet<String> = timeline.clips.iter().map(|clip| clip.id.clone()).collect();
        let mut changed = false;
        for id in ids {
            // Being put back, it is turned off again: the answer to that decides
            if let Some(found) = self.overrides.get_mut(&id) {
                if matches!(found.state, State::Restoring(_)) {
                    found.state = State::Asked(now);
                    found.tries = 0;
                    self.send_override(&id, false, now);
                    self.quiet_sound(&id);
                }
                continue;
            }
            let Some(sound) = self.sounds.get(&id) else {
                continue;
            };
            let own = OwnOptions {
                stops_others: sound.stop_other_sounds,
                mutes_others: sound.mute_other_sounds,
                loops: sound.looped,
            };
            if !own.any() {
                continue;
            }
            self.overrides.insert(id.clone(), Override::new(own, State::Asked(now)));
            self.send_override(&id, false, now);
            self.quiet_sound(&id);
            changed = true;
        }
        if changed {
            self.overrides_changed();
        }
    }

    /// `back`: as Voicemod had them; else turned off
    fn send_override(&mut self, id: &str, back: bool, now: Instant) {
        let Some(found) = self.overrides.get_mut(id) else {
            return;
        };
        // One unanswered for so long never will be
        found.sent.retain(|(_, at)| now < *at + RESTORE_RETRY);
        found.sent.push_back((back, now));
        let own = found.own;
        self.request(Request::OverrideSoundOptions {
            id: id.to_string(),
            stops_others: own.stops_others.then_some(back),
            mutes_others: own.mutes_others.then_some(back),
            loops: own.loops.then_some(back),
            restore: back,
        });
    }

    /// What plays follows the options the macro gives the sound: it no longer stops, mutes or loops
    fn quiet_sound(&mut self, id: &str) {
        if let Some(sound) = self.sounds.get_mut(id) {
            sound.stop_other_sounds = false;
            sound.mute_other_sounds = false;
            sound.looped = false;
        }
    }

    /// As Voicemod has them again, or still
    fn own_sound(&mut self, id: &str, own: OwnOptions) {
        if let Some(sound) = self.sounds.get_mut(id) {
            sound.stop_other_sounds |= own.stops_others;
            sound.mute_other_sounds |= own.mutes_others;
            sound.looped |= own.loops;
        }
    }

    /// Voicemod answered the oldest change sent for this sound, turning off or putting back
    pub(super) fn override_applied(&mut self, sound: &str, changed: bool, now: Instant) {
        let Some(found) = self.overrides.get_mut(sound) else {
            return;
        };
        let Some((back, _)) = found.sent.pop_front() else {
            return;
        };
        // A change sent after it is what Voicemod has in the end: its answer decides
        if !found.sent.is_empty() {
            return;
        }
        let own = found.own;
        match (back, changed) {
            (false, true) => {
                if matches!(found.state, State::Asked(_)) {
                    found.state = State::Applied;
                }
            }
            (false, false) => {
                self.overrides.remove(sound);
                self.own_sound(sound, own);
                self.overrides_changed();
                self.outputs.push(Output::Log(format!(
                    "Voicemod refused to change {sound} for a macro: it plays with its own options"
                )));
            }
            (true, true) => {
                self.overrides.remove(sound);
                self.restored.insert(sound.to_string(), (own, now));
                self.overrides_changed();
            }
            // Sent again later
            (true, false) => {
                self.outputs.push(Output::Log(format!(
                    "Voicemod refused to put back {sound}'s options: trying again"
                )));
            }
        }
    }

    /// A clip of this sound waits for its options to change, for a while at most
    pub(super) fn override_pending(&self, sound: &str, now: Instant) -> bool {
        self.overrides
            .get(sound)
            .is_some_and(|found| matches!(found.state, State::Asked(asked) if now < asked + OVERRIDE_WAIT))
    }

    pub(super) fn override_asked(&self, sound: &str) -> bool {
        self.overrides
            .get(sound)
            .is_some_and(|found| matches!(found.state, State::Asked(_)))
    }

    /// It mutes the other sounds while it plays, as Voicemod has it still: its change not answered for a while
    pub(super) fn mute_pending(&self, sound: &str, now: Instant) -> bool {
        self.override_pending(sound, now) && self.overrides.get(sound).is_some_and(|found| found.own.mutes_others)
    }

    pub(super) fn mute_asked(&self, sound: &str) -> bool {
        self.override_asked(sound) && self.overrides.get(sound).is_some_and(|found| found.own.mutes_others)
    }

    /// It stops or mutes the other sounds, as Voicemod had it
    pub(super) fn quiets_others(&self, sound: &str) -> bool {
        self.overrides
            .get(sound)
            .is_some_and(|found| found.own.stops_others || found.own.mutes_others)
    }

    /// It looped, as Voicemod had it
    pub(super) fn overridden_loop(&self, sound: &str) -> bool {
        self.overrides.get(sound).is_some_and(|found| found.own.loops)
    }

    /// One Voicemod did not answer in time is waited for no more
    pub(super) fn expire_overrides(&mut self, now: Instant) {
        for found in self.overrides.values_mut() {
            if matches!(found.state, State::Asked(asked) if now >= asked + OVERRIDE_WAIT) {
                found.state = State::Applied;
            }
        }
        self.restored.retain(|_, (_, at)| now < *at + RESTORED_HOLD);
    }

    pub(super) fn override_wake(&self) -> Option<Instant> {
        let ready = self.single_stop && self.voicemod == VoicemodStatus::Connected;
        self.overrides
            .values()
            .filter_map(|found| match found.state {
                State::Asked(asked) => Some(asked + OVERRIDE_WAIT),
                State::Restoring(since) if ready => Some(since + RESTORE_RETRY),
                _ => None,
            })
            .min()
    }

    /// The options of the sounds no macro running uses, once quiet, as Voicemod had them; again if not confirmed, a
    /// few times at most. One Voicemod no longer lists is given up.
    pub(super) fn restore_sounds(&mut self, now: Instant) {
        if self.overrides.is_empty() || !self.single_stop || self.voicemod != VoicemodStatus::Connected {
            return;
        }
        let used: HashSet<String> = self
            .running
            .as_ref()
            .and_then(|running| running.item.timeline.as_ref())
            .map(|timeline| timeline.clips.iter().map(|clip| clip.id.clone()).collect())
            .unwrap_or_default();
        let due: Vec<(String, OwnOptions, u8)> = self
            .overrides
            .iter()
            .filter(|(id, found)| match found.state {
                State::Applied => !used.contains(*id) && self.playback.state(id).is_none(),
                State::Restoring(since) => now >= since + RESTORE_RETRY,
                State::Asked(_) => false,
            })
            .map(|(id, found)| (id.clone(), found.own, found.tries))
            .collect();
        let mut given_up = false;
        for (id, own, tries) in due {
            // The soundboards known, a sound they lack is gone from Voicemod
            let gone = !self.sounds.is_empty() && !self.sounds.contains_key(&id);
            if gone || tries >= RESTORE_TRIES {
                self.overrides.remove(&id);
                given_up = true;
                self.outputs.push(Output::Log(format!(
                    "cannot put back {id}'s options in Voicemod: given up"
                )));
                continue;
            }
            self.send_override(&id, true, now);
            self.own_sound(&id, own);
            if let Some(found) = self.overrides.get_mut(&id) {
                found.state = State::Restoring(now);
                found.tries += 1;
            }
        }
        if given_up {
            self.overrides_changed();
        }
    }

    /// The bridge came back: what was being put back is sent again at once, and no answer comes for what went before
    pub(super) fn overrides_reconnected(&mut self) {
        for found in self.overrides.values_mut() {
            found.sent.clear();
            if matches!(found.state, State::Restoring(_)) {
                found.state = State::Applied;
            }
        }
    }

    /// Voicemod's soundboards came: a sound a macro changed keeps what the macro gives it, one put back a moment ago
    /// what Voicemod has again, until a read shows it so
    pub(super) fn keep_override_flags(&mut self) {
        let changed: Vec<String> = self
            .overrides
            .iter()
            .filter(|(_, found)| !matches!(found.state, State::Restoring(_)))
            .map(|(id, _)| id.clone())
            .collect();
        for id in changed {
            self.quiet_sound(&id);
        }
        let restored: Vec<(String, OwnOptions)> =
            self.restored.iter().map(|(id, (own, _))| (id.clone(), *own)).collect();
        for (id, own) in restored {
            // Read after Voicemod put them back: from then on, what it tells holds, the user's own changes included
            let back = self.sounds.get(&id).is_some_and(|sound| {
                (!own.stops_others || sound.stop_other_sounds)
                    && (!own.mutes_others || sound.mute_other_sounds)
                    && (!own.loops || sound.looped)
            });
            if back {
                self.restored.remove(&id);
            } else {
                self.own_sound(&id, own);
            }
        }
    }

    /// Changed by an app closed before it put them back, from the file it kept: put back once the bridge is there
    pub fn left_overrides(&mut self, left: HashMap<String, OwnOptions>) {
        for (id, own) in left {
            self.overrides.entry(id).or_insert(Override::new(own, State::Applied));
        }
    }

    fn overrides_changed(&mut self) {
        let kept = self
            .overrides
            .iter()
            .map(|(id, found)| (id.clone(), found.own))
            .collect();
        self.outputs.push(Output::Overrides(kept));
    }
}
