//! What a pad does when it is pressed again while its sound plays: the sound's own rule in Voicemod, or one the user
//! picks. Voicemod's API can only press a sound and stop every sound, so each choice is made of those two, and what it
//! can do depends on the sound's own mode (docs/project.md, section 8).

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

pub fn outcome(choice: PressAgain, mode: PlaybackMode) -> Outcome {
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
    Outcome { does, stops_all }
}

/// What goes to Voicemod for a pad
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// A click on the sound in Voicemod
    KeyDown,
    /// Ends a sound that plays while held
    KeyUp,
    StopAll,
}

/// For a press of the pad, by what it does and the sound's mode; `active` when the sound plays or is paused.
pub fn on_press(does: PressAgain, mode: PlaybackMode, active: bool) -> &'static [Command] {
    use Command::*;
    use PlaybackMode::*;
    use PressAgain::*;
    if !active {
        return &[KeyDown];
    }
    match (does, mode) {
        // The first click stops it, the second starts it from the start
        (Restart | Hold, PlayStop) => &[KeyDown, KeyDown],
        (Restart | Hold, PlayPause | PlayOverlap) => &[StopAll, KeyDown],
        (Stop, PlayRestart | PlayOverlap | PlayPause) => &[StopAll],
        _ => &[KeyDown],
    }
}

/// For the release of the pad, sent only if the sound still plays.
pub fn on_release(does: PressAgain, mode: PlaybackMode) -> Option<Command> {
    match (does, mode) {
        (PressAgain::Voicemod | PressAgain::Hold, PlaybackMode::PlayLoopOnPress) => Some(Command::KeyUp),
        (PressAgain::Hold, PlaybackMode::PlayStop) => Some(Command::KeyDown),
        (PressAgain::Hold, _) => Some(Command::StopAll),
        _ => None,
    }
}

/// Every choice's outcome for each of Voicemod's modes, for the window to tell what a choice does to a sound.
#[tauri::command]
pub fn press_outcomes() -> HashMap<PlaybackMode, HashMap<PressAgain, Outcome>> {
    let outcomes = |mode| CHOICES.map(|choice| (choice, outcome(choice, mode))).into();
    MODES.map(|mode| (mode, outcomes(mode))).into()
}

/// A pad's choice. One this app does not know, from a newer one, reads as none: the pad follows the one for every pad.
pub fn known<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<PressAgain>, D::Error> {
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(value.and_then(|value| PressAgain::deserialize(value).ok()))
}

/// The choice for every pad, as in Voicemod when this app does not know it.
pub fn known_or_default<'de, D: Deserializer<'de>>(deserializer: D) -> Result<PressAgain, D::Error> {
    Ok(known(deserializer)?.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use Command::*;
    use PlaybackMode::*;

    #[test]
    fn as_in_voicemod_changes_nothing() {
        for mode in MODES {
            let does = outcome(PressAgain::Voicemod, mode).does;
            assert_eq!(does, PressAgain::Voicemod);
            assert_eq!(on_press(does, mode, true), [KeyDown], "{mode:?}");
        }
        assert_eq!(on_release(PressAgain::Voicemod, PlayLoopOnPress), Some(KeyUp));
        assert_eq!(on_release(PressAgain::Voicemod, PlayRestart), None);
    }

    #[test]
    fn a_sound_not_playing_starts_with_one_click() {
        for choice in CHOICES {
            for mode in [PlayRestart, PlayStop, PlayPause] {
                assert_eq!(on_press(outcome(choice, mode).does, mode, false), [KeyDown]);
            }
        }
    }

    #[test]
    fn restart_clicks_twice_where_a_click_stops_it_and_stops_all_where_it_pauses() {
        let restart = |mode| {
            let outcome = outcome(PressAgain::Restart, mode);
            (outcome.stops_all, on_press(outcome.does, mode, true))
        };
        assert_eq!(restart(PlayRestart), (false, &[KeyDown][..]));
        assert_eq!(restart(PlayStop), (false, &[KeyDown, KeyDown][..]));
        assert_eq!(restart(PlayPause), (true, &[StopAll, KeyDown][..]));
        assert_eq!(restart(PlayOverlap), (true, &[StopAll, KeyDown][..]));
        // A sound that plays while held starts from the start at each press already
        assert_eq!(outcome(PressAgain::Restart, PlayLoopOnPress).does, PressAgain::Voicemod);
    }

    #[test]
    fn overlap_and_pause_only_where_voicemod_does_them() {
        assert_eq!(outcome(PressAgain::Overlap, PlayOverlap).does, PressAgain::Overlap);
        assert_eq!(outcome(PressAgain::Overlap, PlayRestart).does, PressAgain::Restart);
        assert_eq!(
            outcome(PressAgain::Overlap, PlayPause),
            Outcome {
                does: PressAgain::Restart,
                stops_all: true
            }
        );
        assert_eq!(outcome(PressAgain::Pause, PlayPause).does, PressAgain::Pause);
        for mode in [PlayRestart, PlayStop, PlayOverlap, PlayLoopOnPress] {
            assert_eq!(outcome(PressAgain::Pause, mode).does, PressAgain::Voicemod, "{mode:?}");
        }
    }

    #[test]
    fn stop_stops_only_that_sound_where_a_click_does() {
        let stop = |mode| {
            let outcome = outcome(PressAgain::Stop, mode);
            (
                outcome.stops_all,
                on_press(outcome.does, mode, true),
                on_release(outcome.does, mode),
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
            let outcome = outcome(PressAgain::Hold, mode);
            (outcome.stops_all, on_release(outcome.does, mode))
        };
        assert_eq!(hold(PlayLoopOnPress), (false, Some(KeyUp)));
        assert_eq!(hold(PlayStop), (false, Some(KeyDown)));
        assert_eq!(hold(PlayRestart), (true, Some(StopAll)));
        assert_eq!(hold(PlayPause), (true, Some(StopAll)));
        assert_eq!(on_press(PressAgain::Hold, PlayStop, true), [KeyDown, KeyDown]);
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
        let table = serde_json::to_value(press_outcomes()).unwrap();
        assert_eq!(
            table["PlayPause"]["stop"],
            serde_json::json!({ "does": "stop", "stopsAll": true })
        );
        assert_eq!(table["Unknown"]["hold"]["does"], "voicemod");
    }
}
