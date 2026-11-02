//! Voicemod's soundboards and voices, and the pages of 36 pads that show them.

use crate::protocol::PAD_COUNT;
use serde::Deserialize;
use serde::de::{IntoDeserializer, value};
use std::collections::HashSet;

const PADS: usize = PAD_COUNT as usize;
pub const FAVOURITES_ID: &str = "favourites";
const FAVOURITES_NAME: &str = "Favourites";
pub const VOICES_ID: &str = "voices";
const VOICES_NAME: &str = "Voices";

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Soundboard {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub sounds: Vec<Sound>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Sound {
    pub id: String,
    pub name: String,
    /// False for the sounds the user's licence does not include
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
    #[serde(default)]
    pub playback_mode: PlaybackMode,
    #[serde(default, rename = "loop")]
    pub looped: bool,
    #[serde(default)]
    pub stop_other_sounds: bool,
    /// The microphone is silenced while it plays
    #[serde(default)]
    pub mute_voice: bool,
    #[serde(default)]
    pub favorited: bool,
}

fn enabled_by_default() -> bool {
    true
}

/// What a press does to a sound that is already playing (docs/project.md, section 8).
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
pub enum PlaybackMode {
    /// Starts it again
    #[default]
    PlayRestart,
    /// Adds another instance
    PlayOverlap,
    PlayStop,
    /// Pauses it; the next press resumes it
    PlayPause,
    /// The sound plays only while its key is held
    PlayLoopOnPress,
    /// A mode added after this app
    #[serde(other)]
    Unknown,
}

impl PlaybackMode {
    /// From its name, as in Voicemod's API.
    pub fn from_name(name: &str) -> Self {
        Self::deserialize(IntoDeserializer::<value::Error>::into_deserializer(name)).unwrap_or(Self::Unknown)
    }
}

/// One of Voicemod's voices, as `getVoices` lists them
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Voice {
    pub id: String,
    #[serde(rename = "friendlyName")]
    pub name: String,
    /// False for the voices the user's licence does not include
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
    #[serde(default)]
    pub favorited: bool,
}

/// What a pad does
#[derive(Clone, Debug, PartialEq)]
pub enum Pad {
    Sound(Sound),
    /// Loads the voice into Voicemod's voice changer
    Voice(Voice),
}

impl Pad {
    pub fn name(&self) -> &str {
        match self {
            Self::Sound(sound) => &sound.name,
            Self::Voice(voice) => &voice.name,
        }
    }

    /// Within the user's licence
    pub fn enabled(&self) -> bool {
        match self {
            Self::Sound(sound) => sound.enabled,
            Self::Voice(voice) => voice.enabled,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Page {
    /// The soundboard's id, or FAVOURITES_ID, or VOICES_ID
    pub soundboard_id: String,
    pub name: String,
    /// Which of the soundboard's pages this is, from 1, when it needs more than one
    pub part: usize,
    pub parts: usize,
    /// One entry per pad
    pub pads: Vec<Option<Pad>>,
}

/// The favourite sounds first, then the voices in the licence, then every soundboard in Voicemod's order, 36 pads
/// per page.
pub fn pages(soundboards: &[Soundboard], voices: &[Voice]) -> Vec<Page> {
    let mut seen = HashSet::new();
    let favourites: Vec<Pad> = soundboards
        .iter()
        .flat_map(|soundboard| &soundboard.sounds)
        .filter(|sound| sound.favorited && seen.insert(&sound.id))
        .map(|sound| Pad::Sound(sound.clone()))
        .collect();

    let mut pages = Vec::new();
    if !favourites.is_empty() {
        add_pages(&mut pages, FAVOURITES_ID, FAVOURITES_NAME, favourites);
    }
    // Only those that can be used: a free licence would fill pages with locked voices. The favourites first.
    let mut usable: Vec<&Voice> = voices.iter().filter(|voice| voice.enabled).collect();
    usable.sort_by_key(|voice| !voice.favorited);
    if !usable.is_empty() {
        let pads = usable.into_iter().map(|voice| Pad::Voice(voice.clone())).collect();
        add_pages(&mut pages, VOICES_ID, VOICES_NAME, pads);
    }
    for soundboard in soundboards {
        let pads = soundboard.sounds.iter().cloned().map(Pad::Sound).collect();
        add_pages(&mut pages, &soundboard.id, &soundboard.name, pads);
    }
    pages
}

fn add_pages(pages: &mut Vec<Page>, soundboard_id: &str, name: &str, pads: Vec<Pad>) {
    // An empty soundboard still gets its (empty) page, as in Voicemod's list
    let parts = pads.len().div_ceil(PADS).max(1);
    let mut pads = pads.into_iter();
    for part in 0..parts {
        let mut page_pads: Vec<Option<Pad>> = pads.by_ref().take(PADS).map(Some).collect();
        page_pads.resize(PADS, None);
        pages.push(Page {
            soundboard_id: soundboard_id.to_string(),
            name: name.to_string(),
            part: part + 1,
            parts,
            pads: page_pads,
        });
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn sound(id: &str) -> Sound {
        Sound {
            id: id.to_string(),
            name: id.to_uppercase(),
            enabled: true,
            playback_mode: PlaybackMode::PlayRestart,
            looped: false,
            stop_other_sounds: false,
            mute_voice: false,
            favorited: false,
        }
    }

    pub fn soundboard(id: &str, sounds: Vec<Sound>) -> Soundboard {
        Soundboard {
            id: id.to_string(),
            name: id.to_uppercase(),
            sounds,
        }
    }

    #[test]
    fn reads_voicemod_soundboards() {
        let json = r#"[{"enabled": true, "id": "b1", "isCustom": true, "name": "My first soundboard", "sounds": [
            {"enabled": false, "favorited": true, "id": "s1", "loop": true, "muteVoice": false, "name": "Honk",
             "playbackMode": "PlayLoopOnPress", "stopOtherSounds": true, "imageURL": "https://example.com/honk"},
            {"id": "s2", "name": "Future", "playbackMode": "PlaySomethingNew"}]}]"#;
        let soundboards: Vec<Soundboard> = serde_json::from_str(json).unwrap();
        let [honk, future] = &soundboards[0].sounds[..] else {
            panic!("two sounds")
        };
        assert!(!honk.enabled && honk.favorited && honk.looped && honk.stop_other_sounds);
        assert_eq!(honk.playback_mode, PlaybackMode::PlayLoopOnPress);
        assert!(future.enabled);
        assert_eq!(future.playback_mode, PlaybackMode::Unknown);
    }

    #[test]
    fn splits_soundboards_into_pages_of_36() {
        let big: Vec<Sound> = (0..40).map(|i| sound(&format!("s{i}"))).collect();
        let pages = pages(&[soundboard("big", big), soundboard("empty", vec![])], &[]);

        let name = |page: &Page, pad: usize| page.pads[pad].as_ref().map(|pad| pad.name().to_string());
        assert_eq!(pages.len(), 3);
        assert_eq!((pages[0].part, pages[0].parts), (1, 2));
        assert_eq!(name(&pages[0], 35).as_deref(), Some("S35"));
        assert_eq!(name(&pages[1], 3).as_deref(), Some("S39"));
        assert!(pages[1].pads[4].is_none());
        assert_eq!(pages[1].pads.len(), PADS);
        assert_eq!((pages[2].soundboard_id.as_str(), pages[2].parts), ("empty", 1));
        assert!(pages[2].pads.iter().all(Option::is_none));
    }

    #[test]
    fn puts_each_favourite_once_on_the_first_page() {
        let favourite = Sound {
            favorited: true,
            ..sound("fav")
        };
        let pages = pages(
            &[
                soundboard("a", vec![sound("x"), favourite.clone()]),
                soundboard("b", vec![favourite]),
            ],
            &[],
        );

        assert_eq!(pages.len(), 3);
        assert_eq!(pages[0].name, FAVOURITES_NAME);
        let favourites: Vec<_> = pages[0].pads.iter().flatten().map(Pad::name).collect();
        assert_eq!(favourites, ["FAV"]);
        assert_eq!(pages[1].soundboard_id, "a");
    }

    pub fn voice(id: &str) -> Voice {
        Voice {
            id: id.to_string(),
            name: id.to_uppercase(),
            enabled: true,
            favorited: false,
        }
    }

    #[test]
    fn puts_the_usable_voices_after_the_favourites() {
        let json = r#"[{"id": "v1", "friendlyName": "Dark", "enabled": false, "favorited": false, "isNew": false},
            {"id": "v2", "friendlyName": "Banana", "enabled": true, "favorited": false}]"#;
        let mut voices: Vec<Voice> = serde_json::from_str(json).unwrap();
        voices.push(Voice {
            favorited: true,
            ..voice("v3")
        });
        let favourite = Sound {
            favorited: true,
            ..sound("fav")
        };
        let pages = pages(&[soundboard("a", vec![favourite])], &voices);

        assert_eq!(pages[1].soundboard_id, VOICES_ID);
        let names: Vec<_> = pages[1].pads.iter().flatten().map(Pad::name).collect();
        assert_eq!(names, ["V3", "Banana"]);
        assert_eq!(pages[2].soundboard_id, "a");
    }
}
