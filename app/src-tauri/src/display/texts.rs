//! The words on the display, in the simulator's languages: English and Italian, as picked in the settings.

use crate::soundboard::{Action, PlaybackMode};

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
    pub unreachable: &'static str,
    pub connecting: &'static str,
    /// Voicemod refused the app's key: a newer app has a valid one
    pub update_app: &'static str,
    pub paused: &'static str,
    pub looped: &'static str,
    pub mute_me: &'static str,
    pub mic_off: &'static str,
    pub stop: &'static str,
    pub voice_off: &'static str,
    pub hear_me: &'static str,
    pub row: &'static str,
    pub empty: &'static str,
    pub locked: &'static str,
    pub stops_others: &'static str,
    /// What an action pad does, when peeked
    pub action: &'static str,
    /// Before a page's name, on a pad that opens it
    pub go_to: &'static str,
    modes: Modes,
    actions: Actions,
}

/// The playback modes' names
struct Modes {
    restart: &'static str,
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
}

pub const ENGLISH: Texts = Texts {
    ready: "Ready",
    no_sound: "No sound",
    community: "Community",
    soundboards: "SOUNDBOARDS",
    favourites: "★ Favourites",
    voices: "Voices",
    voice: "VOICE",
    unreachable: "UNREACHABLE",
    connecting: "CONNECTING...",
    update_app: "UPDATE THE APP",
    paused: "PAUSED",
    looped: "LOOP",
    mute_me: "MUTE ME",
    mic_off: "MIC OFF",
    stop: "STOP",
    voice_off: "VOICE OFF",
    hear_me: "HEAR ME",
    row: "ROW",
    empty: "EMPTY",
    locked: "NOT IN LICENCE",
    stops_others: "STOPS OTHERS",
    action: "ACTION",
    go_to: "Go to",
    modes: Modes {
        restart: "ONE-SHOT",
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
    unreachable: "NON RAGGIUNGIBILE",
    connecting: "CONNESSIONE...",
    update_app: "AGGIORNA L'APP",
    paused: "PAUSA",
    looped: "LOOP",
    mute_me: "MUTO",
    mic_off: "MIC OFF",
    stop: "STOP",
    voice_off: "VOCE OFF",
    hear_me: "ASCOLTO",
    row: "RIGA",
    empty: "VUOTO",
    locked: "NON IN LICENZA",
    stops_others: "FERMA GLI ALTRI",
    action: "AZIONE",
    go_to: "Vai a",
    modes: Modes {
        restart: "SINGOLO",
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
