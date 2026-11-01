//! Voicemod's soundboards, and the pages of 36 sounds the pads show them on.

use crate::protocol::PAD_COUNT;
use serde::Deserialize;
use std::collections::HashSet;

const PADS: usize = PAD_COUNT as usize;
const FAVOURITES_ID: &str = "favourites";
const FAVOURITES_NAME: &str = "Favourites";

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

#[derive(Clone, Debug, PartialEq)]
pub struct Page {
    pub soundboard_id: String,
    pub name: String,
    /// Which of the soundboard's pages this is, from 1, when it needs more than one
    pub part: usize,
    pub parts: usize,
    /// One entry per pad
    pub sounds: Vec<Option<Sound>>,
}

/// The favourite sounds first, then every soundboard in Voicemod's order, 36 sounds per page.
pub fn pages(soundboards: &[Soundboard]) -> Vec<Page> {
    let mut seen = HashSet::new();
    let favourites: Vec<Sound> = soundboards
        .iter()
        .flat_map(|soundboard| &soundboard.sounds)
        .filter(|sound| sound.favorited && seen.insert(&sound.id))
        .cloned()
        .collect();

    let mut pages = Vec::new();
    if !favourites.is_empty() {
        add_pages(&mut pages, FAVOURITES_ID, FAVOURITES_NAME, &favourites);
    }
    for soundboard in soundboards {
        add_pages(&mut pages, &soundboard.id, &soundboard.name, &soundboard.sounds);
    }
    pages
}

fn add_pages(pages: &mut Vec<Page>, soundboard_id: &str, name: &str, sounds: &[Sound]) {
    // An empty soundboard still gets its (empty) page, as in Voicemod's list
    let parts = sounds.len().div_ceil(PADS).max(1);
    for part in 0..parts {
        let chunk = sounds.iter().skip(part * PADS).take(PADS);
        let mut page_sounds: Vec<Option<Sound>> = chunk.cloned().map(Some).collect();
        page_sounds.resize(PADS, None);
        pages.push(Page {
            soundboard_id: soundboard_id.to_string(),
            name: name.to_string(),
            part: part + 1,
            parts,
            sounds: page_sounds,
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
        let pages = pages(&[soundboard("big", big), soundboard("empty", vec![])]);

        assert_eq!(pages.len(), 3);
        assert_eq!((pages[0].part, pages[0].parts), (1, 2));
        assert_eq!(pages[0].sounds[35].as_ref().unwrap().id, "s35");
        assert_eq!(pages[1].sounds[3].as_ref().unwrap().id, "s39");
        assert!(pages[1].sounds[4].is_none());
        assert_eq!(pages[1].sounds.len(), PADS);
        assert_eq!((pages[2].soundboard_id.as_str(), pages[2].parts), ("empty", 1));
        assert!(pages[2].sounds.iter().all(Option::is_none));
    }

    #[test]
    fn puts_each_favourite_once_on_the_first_page() {
        let favourite = Sound {
            favorited: true,
            ..sound("fav")
        };
        let pages = pages(&[
            soundboard("a", vec![sound("x"), favourite.clone()]),
            soundboard("b", vec![favourite]),
        ]);

        assert_eq!(pages.len(), 3);
        assert_eq!(pages[0].name, FAVOURITES_NAME);
        let favourites: Vec<_> = pages[0]
            .sounds
            .iter()
            .flatten()
            .map(|sound| sound.id.as_str())
            .collect();
        assert_eq!(favourites, ["fav"]);
        assert_eq!(pages[1].soundboard_id, "a");
    }
}
