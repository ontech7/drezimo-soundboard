//! The words on the display, in the simulator's languages: English and Italian, as picked in the settings.

use crate::soundboard::PlaybackMode;

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
    modes: Modes,
}

/// The playback modes' names
struct Modes {
    restart: &'static str,
    overlap: &'static str,
    stop: &'static str,
    pause: &'static str,
    hold: &'static str,
}

impl Texts {
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
    modes: Modes {
        restart: "ONE-SHOT",
        overlap: "OVERLAP",
        stop: "PLAY/STOP",
        pause: "PLAY/PAUSE",
        hold: "HOLD",
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
    modes: Modes {
        restart: "SINGOLO",
        overlap: "SOVRAPPOSTO",
        stop: "PLAY/STOP",
        pause: "PLAY/PAUSA",
        hold: "TIENI PREMUTO",
    },
};
