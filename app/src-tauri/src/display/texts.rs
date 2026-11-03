//! The words on the display, in the simulator's languages: English and Italian, as picked in the settings.

use crate::press_again::PressAgain;
use crate::soundboard::{Action, PlaybackMode};
use std::time::Duration;

pub struct Texts {
    pub ready: &'static str,
    pub no_sound: &'static str,
    /// The name of a sound outside the soundboards, played from Voicemod's Community
    pub community: &'static str,
    pub soundboards: &'static str,
    pub favourites: &'static str,
    pub voices: &'static str,
    /// What a voice pad does, when peeked
    pub voice: &'static str,
    /// Voicemod is not running
    pub open_voicemod: &'static str,
    pub connecting: &'static str,
    /// Voicemod refused the app's key: a newer app has a valid one
    pub update_app: &'static str,
    pub looped: &'static str,
    pub stop: &'static str,
    /// Where the voice loaded shows, while the voice changer is off
    pub voice_off: &'static str,
    pub row: &'static str,
    pub empty: &'static str,
    pub locked: &'static str,
    /// In place of the name, when a pad outside the licence is pressed
    pub pro_only: &'static str,
    pub stops_others: &'static str,
    /// A pad that stops every sound when pressed again, when peeked
    pub stops_all: &'static str,
    /// What an action pad does, when peeked
    pub action: &'static str,
    /// Before a page's name, on a pad that opens it
    pub go_to: &'static str,
    /// What a macro pad does, when peeked
    pub macro_info: &'static str,
    decimal_separator: char,
    modes: Modes,
    actions: Actions,
}

/// The playback modes' names
struct Modes {
    restart: &'static str,
    /// A pad that starts its sound from the start when pressed again
    start_over: &'static str,
    overlap: &'static str,
    stop: &'static str,
    pause: &'static str,
    hold: &'static str,
}

/// The action pads' own names
struct Actions {
    stop_sounds: &'static str,
    mute_mic: &'static str,
    voice_changer: &'static str,
    random_voice: &'static str,
    hear_myself: &'static str,
    mute_for_me: &'static str,
    background: &'static str,
    go_to_page: &'static str,
}

impl Texts {
    pub fn action_name(&self, action: Action) -> &'static str {
        let names = &self.actions;
        match action {
            Action::StopSounds => names.stop_sounds,
            Action::MuteMic => names.mute_mic,
            Action::VoiceChanger => names.voice_changer,
            Action::RandomVoice => names.random_voice,
            Action::HearMyself => names.hear_myself,
            Action::MuteForMe => names.mute_for_me,
            Action::Background => names.background,
            Action::GoToPage => names.go_to_page,
        }
    }

    /// The row overview's heading, such as "ROW 2/6", from row 0.
    pub fn row_heading(&self, row: usize, rows: usize) -> String {
        format!("{} {}/{rows}", self.row, row + 1)
    }

    /// Seconds to a tenth, such as "1.5s".
    pub fn seconds(&self, duration: Duration) -> String {
        format!("{:.1}s", duration.as_secs_f64()).replace('.', &self.decimal_separator.to_string())
    }

    pub fn mode(&self, mode: PlaybackMode) -> &'static str {
        match mode {
            PlaybackMode::PlayRestart => self.modes.restart,
            PlaybackMode::PlayOverlap => self.modes.overlap,
            PlaybackMode::PlayStop => self.modes.stop,
            PlaybackMode::PlayPause => self.modes.pause,
            PlaybackMode::PlayLoopOnPress => self.modes.hold,
            PlaybackMode::Unknown => "",
        }
    }

    /// What a pad pressed again does: the sound's own mode, or what the user picked.
    pub fn press_again(&self, does: PressAgain, mode: PlaybackMode) -> &'static str {
        match does {
            PressAgain::Voicemod => self.mode(mode),
            PressAgain::Restart => self.modes.start_over,
            PressAgain::Overlap => self.modes.overlap,
            PressAgain::Stop => self.modes.stop,
            PressAgain::Pause => self.modes.pause,
            PressAgain::Hold => self.modes.hold,
        }
    }
}

pub const ENGLISH: Texts = Texts {
    ready: "Ready",
    no_sound: "No sound",
    community: "Community",
    soundboards: "SOUNDBOARDS",
    favourites: "★ Favourites",
    voices: "Voices",
    voice: "VOICE",
    open_voicemod: "OPEN IT ON YOUR PC",
    connecting: "CONNECTING",
    update_app: "UPDATE THE APP",
    looped: "LOOP",
    stop: "STOP",
    voice_off: "VOICE OFF",
    row: "ROW",
    empty: "EMPTY",
    locked: "NOT IN LICENCE",
    pro_only: "PRO ONLY",
    stops_others: "STOPS OTHERS",
    stops_all: "STOPS ALL",
    action: "ACTION",
    go_to: "Go to",
    macro_info: "MACRO",
    decimal_separator: '.',
    modes: Modes {
        restart: "ONE-SHOT",
        start_over: "RESTART",
        overlap: "OVERLAP",
        stop: "PLAY/STOP",
        pause: "PLAY/PAUSE",
        hold: "HOLD",
    },
    actions: Actions {
        stop_sounds: "Stop sounds",
        mute_mic: "Mute mic",
        voice_changer: "Voice changer",
        random_voice: "Random voice",
        hear_myself: "Hear myself",
        mute_for_me: "Mute for me",
        background: "Background",
        go_to_page: "Go to page",
    },
};

pub const ITALIAN: Texts = Texts {
    ready: "Pronto",
    no_sound: "Senza suoni",
    community: "Community",
    soundboards: "SOUNDBOARD",
    favourites: "★ Preferiti",
    voices: "Voci",
    voice: "VOCE",
    open_voicemod: "APRILO SUL COMPUTER",
    connecting: "CONNESSIONE",
    update_app: "AGGIORNA L'APP",
    looped: "LOOP",
    stop: "STOP",
    voice_off: "VOCE SPENTA",
    row: "RIGA",
    empty: "VUOTO",
    locked: "NON IN LICENZA",
    pro_only: "SOLO PRO",
    stops_others: "FERMA GLI ALTRI",
    stops_all: "FERMA TUTTI",
    action: "AZIONE",
    go_to: "Vai a",
    macro_info: "MACRO",
    decimal_separator: ',',
    modes: Modes {
        restart: "SINGOLO",
        start_over: "DA CAPO",
        overlap: "SOVRAPPOSTO",
        stop: "PLAY/STOP",
        pause: "PLAY/PAUSA",
        hold: "TIENI PREMUTO",
    },
    actions: Actions {
        stop_sounds: "Ferma tutto",
        mute_mic: "Muto mic",
        voice_changer: "Cambio voce",
        random_voice: "Voce casuale",
        hear_myself: "Ascoltami",
        mute_for_me: "Non sentire",
        background: "Sottofondo",
        go_to_page: "Vai a pagina",
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_seconds_with_the_languages_separator() {
        let time = Duration::from_millis(1500);
        assert_eq!(
            (ENGLISH.seconds(time), ITALIAN.seconds(time)),
            ("1.5s".into(), "1,5s".into())
        );
    }
}
