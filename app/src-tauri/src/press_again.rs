//! What a pad does when it is pressed again while its sound plays: the sound's own rule in Voicemod, or one the user
//! picks. Voicemod's API can only press a sound and stop every sound; its bridge, when found, also stops a single one.
//! Each choice is made of those, and what it can do depends on the sound's own mode (docs/project.md, section 8).

use crate::soundboard::PlaybackMode;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PressAgain {
    /// The sound's own rule in Voicemod
    #[default]
    Voicemod,
    /// From the start
    Restart,
    /// Another copy over the ones playing
    Overlap,
    /// Stops it; the next press starts it from the start
    Stop,
    /// Pauses it; the next press resumes it
    Pause,
    /// Plays only while the pad is held
    Hold,
}

const MODES: [PlaybackMode; 6] = [
    PlaybackMode::PlayRestart,
    PlaybackMode::PlayOverlap,
    PlaybackMode::PlayStop,
    PlaybackMode::PlayPause,
    PlaybackMode::PlayLoopOnPress,
    PlaybackMode::Unknown,
];

const CHOICES: [PressAgain; 6] = [
    PressAgain::Voicemod,
    PressAgain::Restart,
    PressAgain::Overlap,
    PressAgain::Stop,
    PressAgain::Pause,
    PressAgain::Hold,
];

/// What a choice comes to for a sound
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    /// The choice, or what the pad does instead where Voicemod cannot do it
    pub does: PressAgain,
    /// Stopping the sound, or starting it again, stops every other sound too
    pub stops_all: bool,
}

/// With `single_stop` when Voicemod's bridge can stop a single sound.
pub fn outcome(choice: PressAgain, mode: PlaybackMode, single_stop: bool) -> Outcome {
    use PlaybackMode::*;
    use PressAgain::*;
    let (does, stops_all) = match (choice, mode) {
        (Voicemod, _) | (_, Unknown) => (Voicemod, false),
        (Restart | Overlap | Pause, PlayLoopOnPress) => (Voicemod, false),
        (Overlap, PlayOverlap) => (Overlap, false),
        (Pause, PlayPause) => (Pause, false),
        (Pause, _) => (Voicemod, false),
        (Restart | Overlap, PlayRestart | PlayStop) => (Restart, false),
        (Restart | Overlap, PlayPause | PlayOverlap) => (Restart, true),
        (Stop | Hold, PlayStop | PlayLoopOnPress) => (choice, false),
        (Stop | Hold, PlayRestart | PlayOverlap | PlayPause) => (choice, true),
    };
    Outcome {
        does,
        stops_all: stops_all && !single_stop,
    }
}

/// What goes to Voicemod for a pad
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// A click on the sound in Voicemod
    KeyDown,
    /// Ends a sound that plays while held
    KeyUp,
    /// Through Voicemod's bridge
    StopSound,
    StopAll,
}

/// For a press of the pad, by what it does and the sound's mode; `active` when the sound plays or is paused.
pub fn on_press(does: PressAgain, mode: PlaybackMode, active: bool, single_stop: bool) -> &'static [Command] {
    use Command::*;
    use PlaybackMode::*;
    use PressAgain::*;
    if !active {
        return &[KeyDown];
    }
    match (does, mode) {
        (Restart | Hold, PlayStop | PlayPause | PlayOverlap) if single_stop => &[StopSound, KeyDown],
        (Stop, PlayRestart | PlayOverlap | PlayPause) if single_stop => &[StopSound],
        // The first click stops it, the second starts it from the start
        (Restart | Hold, PlayStop) => &[KeyDown, KeyDown],
        (Restart | Hold, PlayPause | PlayOverlap) => &[StopAll, KeyDown],
        (Stop, PlayRestart | PlayOverlap | PlayPause) => &[StopAll],
        _ => &[KeyDown],
    }
}

/// For the release of the pad, sent only if the sound still plays.
pub fn on_release(does: PressAgain, mode: PlaybackMode, single_stop: bool) -> Option<Command> {
    match (does, mode) {
        (PressAgain::Voicemod | PressAgain::Hold, PlaybackMode::PlayLoopOnPress) => Some(Command::KeyUp),
        (PressAgain::Hold, _) if single_stop => Some(Command::StopSound),
        (PressAgain::Hold, PlaybackMode::PlayStop) => Some(Command::KeyDown),
        (PressAgain::Hold, _) => Some(Command::StopAll),
        _ => None,
    }
}

/// Every choice's outcome for each of Voicemod's modes, for the window to tell what a choice does to a sound.
#[tauri::command]
pub fn press_outcomes(single_stop: bool) -> HashMap<PlaybackMode, HashMap<PressAgain, Outcome>> {
    let outcomes = |mode| {
        CHOICES
            .map(|choice| (choice, outcome(choice, mode, single_stop)))
            .into()
    };
    MODES.map(|mode| (mode, outcomes(mode))).into()
}

/// A pad's choice. One this app does not know, from a newer one, reads as none: the pad follows the one for every pad.
pub fn known<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<PressAgain>, D::Error> {
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(value.and_then(|value| PressAgain::deserialize(value).ok()))
}

/// A type's choice, as in Voicemod when this app does not know it.
pub fn known_or_default<'de, D: Deserializer<'de>>(deserializer: D) -> Result<PressAgain, D::Error> {
    Ok(known(deserializer)?.unwrap_or_default())
}

/// The choice for each of Voicemod's playback modes, unless a pad has its own
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ByMode {
    #[serde(deserialize_with = "known_or_default")]
    pub play_restart: PressAgain,
    #[serde(deserialize_with = "known_or_default")]
    pub play_overlap: PressAgain,
    #[serde(deserialize_with = "known_or_default")]
    pub play_pause: PressAgain,
    #[serde(deserialize_with = "known_or_default")]
    pub play_stop: PressAgain,
    #[serde(deserialize_with = "known_or_default")]
    pub play_loop_on_press: PressAgain,
}

impl ByMode {
    /// The modes with a choice of their own, in the window's order
    pub const MODES: [PlaybackMode; 5] = [
        PlaybackMode::PlayRestart,
        PlaybackMode::PlayPause,
        PlaybackMode::PlayStop,
        PlaybackMode::PlayLoopOnPress,
        PlaybackMode::PlayOverlap,
    ];

    pub fn get(&self, mode: PlaybackMode) -> PressAgain {
        match mode {
            PlaybackMode::PlayRestart => self.play_restart,
            PlaybackMode::PlayOverlap => self.play_overlap,
            PlaybackMode::PlayPause => self.play_pause,
            PlaybackMode::PlayStop => self.play_stop,
            PlaybackMode::PlayLoopOnPress => self.play_loop_on_press,
            PlaybackMode::Unknown => PressAgain::Voicemod,
        }
    }

    /// One choice for every pad, from an older app, as the same behaviour for each mode.
    pub fn from_one(choice: PressAgain) -> Self {
        // What the choice came to for that mode, which is not always the choice itself
        let for_mode = |mode| {
            let does = outcome(choice, mode, true).does;
            if voicemods_own(mode) == Some(does) {
                PressAgain::Voicemod
            } else {
                does
            }
        };
        Self {
            play_restart: for_mode(PlaybackMode::PlayRestart),
            play_overlap: for_mode(PlaybackMode::PlayOverlap),
            play_pause: for_mode(PlaybackMode::PlayPause),
            play_stop: for_mode(PlaybackMode::PlayStop),
            play_loop_on_press: for_mode(PlaybackMode::PlayLoopOnPress),
        }
    }
}

/// The choice that does exactly what Voicemod does with a sound of that mode.
pub fn voicemods_own(mode: PlaybackMode) -> Option<PressAgain> {
    match mode {
        PlaybackMode::PlayRestart => Some(PressAgain::Restart),
        PlaybackMode::PlayPause => Some(PressAgain::Pause),
        PlaybackMode::PlayStop => Some(PressAgain::Stop),
        PlaybackMode::PlayLoopOnPress => Some(PressAgain::Hold),
        PlaybackMode::PlayOverlap => Some(PressAgain::Overlap),
        PlaybackMode::Unknown => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Command::*;
    use PlaybackMode::*;

    /// Without Voicemod's bridge
    const ALL: bool = false;
    /// With it
    const SINGLE: bool = true;

    #[test]
    fn as_in_voicemod_changes_nothing() {
        for single_stop in [ALL, SINGLE] {
            for mode in MODES {
                let does = outcome(PressAgain::Voicemod, mode, single_stop).does;
                assert_eq!(does, PressAgain::Voicemod);
                assert_eq!(on_press(does, mode, true, single_stop), [KeyDown], "{mode:?}");
            }
            assert_eq!(
                on_release(PressAgain::Voicemod, PlayLoopOnPress, single_stop),
                Some(KeyUp)
            );
            assert_eq!(on_release(PressAgain::Voicemod, PlayRestart, single_stop), None);
        }
    }

    #[test]
    fn a_sound_not_playing_starts_with_one_click() {
        for choice in CHOICES {
            for mode in [PlayRestart, PlayStop, PlayPause] {
                let does = outcome(choice, mode, SINGLE).does;
                assert_eq!(on_press(does, mode, false, SINGLE), [KeyDown]);
            }
        }
    }

    #[test]
    fn restart_clicks_twice_where_a_click_stops_it_and_stops_all_where_it_pauses() {
        let restart = |mode| {
            let outcome = outcome(PressAgain::Restart, mode, ALL);
            (outcome.stops_all, on_press(outcome.does, mode, true, ALL))
        };
        assert_eq!(restart(PlayRestart), (false, &[KeyDown][..]));
        assert_eq!(restart(PlayStop), (false, &[KeyDown, KeyDown][..]));
        assert_eq!(restart(PlayPause), (true, &[StopAll, KeyDown][..]));
        assert_eq!(restart(PlayOverlap), (true, &[StopAll, KeyDown][..]));
        // A sound that plays while held starts from the start at each press already
        assert_eq!(
            outcome(PressAgain::Restart, PlayLoopOnPress, ALL).does,
            PressAgain::Voicemod
        );
    }

    #[test]
    fn overlap_and_pause_only_where_voicemod_does_them() {
        assert_eq!(outcome(PressAgain::Overlap, PlayOverlap, ALL).does, PressAgain::Overlap);
        assert_eq!(outcome(PressAgain::Overlap, PlayRestart, ALL).does, PressAgain::Restart);
        assert_eq!(
            outcome(PressAgain::Overlap, PlayPause, ALL),
            Outcome {
                does: PressAgain::Restart,
                stops_all: true
            }
        );
        assert_eq!(outcome(PressAgain::Pause, PlayPause, ALL).does, PressAgain::Pause);
        for mode in [PlayRestart, PlayStop, PlayOverlap, PlayLoopOnPress] {
            assert_eq!(
                outcome(PressAgain::Pause, mode, SINGLE).does,
                PressAgain::Voicemod,
                "{mode:?}"
            );
        }
    }

    #[test]
    fn stop_stops_only_that_sound_where_a_click_does() {
        let stop = |mode| {
            let outcome = outcome(PressAgain::Stop, mode, ALL);
            (
                outcome.stops_all,
                on_press(outcome.does, mode, true, ALL),
                on_release(outcome.does, mode, ALL),
            )
        };
        assert_eq!(stop(PlayStop), (false, &[KeyDown][..], None));
        // Held sounds keep playing once let go, until pressed again
        assert_eq!(stop(PlayLoopOnPress), (false, &[KeyDown][..], None));
        assert_eq!(stop(PlayRestart), (true, &[StopAll][..], None));
        assert_eq!(stop(PlayPause), (true, &[StopAll][..], None));
    }

    #[test]
    fn hold_ends_the_sound_on_release() {
        let hold = |mode| {
            let outcome = outcome(PressAgain::Hold, mode, ALL);
            (outcome.stops_all, on_release(outcome.does, mode, ALL))
        };
        assert_eq!(hold(PlayLoopOnPress), (false, Some(KeyUp)));
        assert_eq!(hold(PlayStop), (false, Some(KeyDown)));
        assert_eq!(hold(PlayRestart), (true, Some(StopAll)));
        assert_eq!(hold(PlayPause), (true, Some(StopAll)));
        assert_eq!(on_press(PressAgain::Hold, PlayStop, true, ALL), [KeyDown, KeyDown]);
    }

    #[test]
    fn the_bridge_stops_that_sound_alone() {
        for mode in [PlayRestart, PlayOverlap, PlayStop, PlayPause] {
            for choice in [PressAgain::Restart, PressAgain::Stop, PressAgain::Hold] {
                assert!(!outcome(choice, mode, SINGLE).stops_all, "{choice:?} {mode:?}");
            }
            assert_eq!(on_release(PressAgain::Hold, mode, SINGLE), Some(StopSound), "{mode:?}");
        }
        for mode in [PlayRestart, PlayOverlap, PlayPause] {
            assert_eq!(on_press(PressAgain::Stop, mode, true, SINGLE), [StopSound], "{mode:?}");
        }
        // Its own click stops it, and starts it if it just ended unseen
        assert_eq!(on_press(PressAgain::Stop, PlayStop, true, SINGLE), [KeyDown]);
        // From the start: stopped first, then pressed
        for mode in [PlayStop, PlayPause, PlayOverlap] {
            assert_eq!(on_press(PressAgain::Restart, mode, true, SINGLE), [StopSound, KeyDown]);
        }
        assert_eq!(on_press(PressAgain::Restart, PlayRestart, true, SINGLE), [KeyDown]);
        // A held sound keeps its own key-down and key-up
        assert_eq!(on_press(PressAgain::Stop, PlayLoopOnPress, true, SINGLE), [KeyDown]);
        assert_eq!(on_release(PressAgain::Hold, PlayLoopOnPress, SINGLE), Some(KeyUp));
    }

    #[test]
    fn a_choice_from_a_newer_app_is_left_out() {
        let read = |json| known(&mut serde_json::Deserializer::from_str(json)).unwrap();
        assert_eq!(read(r#""pause""#), Some(PressAgain::Pause));
        assert_eq!(read(r#""bounce""#), None);
        assert_eq!(read("null"), None);
    }

    #[test]
    fn the_windows_table_names_modes_and_choices_as_the_json_does() {
        let table = serde_json::to_value(press_outcomes(ALL)).unwrap();
        assert_eq!(
            table["PlayPause"]["stop"],
            serde_json::json!({ "does": "stop", "stopsAll": true })
        );
        assert_eq!(table["Unknown"]["hold"]["does"], "voicemod");
        let single = serde_json::to_value(press_outcomes(SINGLE)).unwrap();
        assert_eq!(single["PlayPause"]["stop"]["stopsAll"], false);
    }

    #[test]
    fn one_choice_for_every_pad_becomes_the_same_behaviour_per_mode() {
        let stop = ByMode::from_one(PressAgain::Stop);
        // Stop applies to every mode; where it is Voicemod's own, it is stored as Voicemod
        assert_eq!(stop.play_restart, PressAgain::Stop);
        assert_eq!(stop.play_overlap, PressAgain::Stop);
        assert_eq!(stop.play_pause, PressAgain::Stop);
        assert_eq!(stop.play_stop, PressAgain::Voicemod);
        assert_eq!(stop.play_loop_on_press, PressAgain::Stop);
        let pause = ByMode::from_one(PressAgain::Pause);
        assert_eq!(
            pause,
            ByMode::default(),
            "pause only exists where Voicemod already pauses"
        );
        let overlap = ByMode::from_one(PressAgain::Overlap);
        assert_eq!(overlap.play_overlap, PressAgain::Voicemod);
        assert_eq!(
            overlap.play_restart,
            PressAgain::Voicemod,
            "overlap on a restart sound is a restart, Voicemod's own"
        );
    }

    #[test]
    fn the_migration_changes_no_pad() {
        for choice in CHOICES {
            let by_mode = ByMode::from_one(choice);
            for mode in MODES {
                for single in [ALL, SINGLE] {
                    let before = outcome(choice, mode, single);
                    let after = outcome(by_mode.get(mode), mode, single);
                    for active in [false, true] {
                        assert_eq!(
                            on_press(before.does, mode, active, single),
                            on_press(after.does, mode, active, single),
                            "{choice:?} {mode:?}"
                        );
                    }
                    assert_eq!(
                        on_release(before.does, mode, single),
                        on_release(after.does, mode, single),
                        "{choice:?} {mode:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn an_unknown_choice_for_a_mode_reads_as_voicemod() {
        let by_mode: ByMode = serde_json::from_str(r#"{"playPause": "stop", "playStop": "fly"}"#).unwrap();
        assert_eq!(by_mode.play_pause, PressAgain::Stop);
        assert_eq!(by_mode.play_stop, PressAgain::Voicemod);
        assert_eq!(by_mode.get(PlaybackMode::Unknown), PressAgain::Voicemod);
    }
}
