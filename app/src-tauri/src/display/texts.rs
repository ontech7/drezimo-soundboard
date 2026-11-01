//! The words on the display, in the simulator's languages. The friend's unit shows Italian.

use crate::soundboard::PlaybackMode;

pub struct Texts {
    pub ready: &'static str,
    pub no_sound: &'static str,
    pub soundboards: &'static str,
    pub favourites: &'static str,
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
    /// The language of the system, when there are words for it.
    pub fn for_system() -> &'static Self {
        let italian = sys_locale::get_locale().is_some_and(|locale| locale.starts_with("it"));
        if italian { &ITALIAN } else { &ENGLISH }
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
    soundboards: "SOUNDBOARDS",
    favourites: "★ Favourites",
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
    soundboards: "SOUNDBOARD",
    favourites: "★ Preferiti",
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
