//! Voicemod's soundboards and voices, the user's own pages, and the pages of 36 pads that show them all.

use crate::custom_pages::{Content, CustomPad, CustomPage};
use crate::display::texts::Texts;
use crate::macros::Macro;
use crate::press_again::PressAgain;
use crate::protocol::PAD_COUNT;
use serde::de::{IntoDeserializer, value};
use serde::{Deserialize, Serialize};
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
    /// Made by the user in Voicemod, rather than one of Voicemod's own
    #[serde(default, rename = "isCustom")]
    pub custom: bool,
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
    /// Where Voicemod keeps its picture, named by the picture's checksum
    #[serde(default, rename = "imageURL")]
    pub image_url: String,
}

fn enabled_by_default() -> bool {
    true
}

/// What a press does to a sound that is already playing (docs/project.md, section 8).
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Hash, Serialize)]
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
    /// As a sound's options name it in Voicemod's bridge
    pub fn option_name(self) -> Option<&'static str> {
        Some(match self {
            Self::PlayRestart => "playRestart",
            Self::PlayOverlap => "playOverlap",
            Self::PlayStop => "playStop",
            Self::PlayPause => "playPause",
            Self::PlayLoopOnPress => "playLoopOnPress",
            Self::Unknown => return None,
        })
    }

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
    #[serde(default, rename = "imageURL")]
    pub image_url: String,
}

/// Which of Voicemod's pages the board shows besides the user's own: all of them unless the user picks
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ShownPages {
    pub favourites: bool,
    pub voices: bool,
    /// Only the soundboards the user made in Voicemod, none of Voicemod's own
    pub only_mine: bool,
    /// Soundboards left out, by id
    pub hidden: Vec<String>,
}

impl Default for ShownPages {
    fn default() -> Self {
        Self {
            favourites: true,
            voices: true,
            only_mine: false,
            hidden: Vec::new(),
        }
    }
}

/// More ids than any Voicemod has soundboards
const MAX_HIDDEN: usize = 500;

impl ShownPages {
    pub fn checked(mut self) -> Self {
        self.hidden.truncate(MAX_HIDDEN);
        self
    }

    pub fn shows(&self, soundboard: &Soundboard) -> bool {
        (soundboard.custom || !self.only_mine) && !self.hidden.contains(&soundboard.id)
    }
}

/// What a pad of the user's own pages does besides sounds and voices: what the function keys do, and more
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    StopSounds,
    /// Voicemod's microphone, on and off
    MuteMic,
    VoiceChanger,
    /// One of the voices in the licence, other than the current one
    RandomVoice,
    HearMyself,
    MuteForMe,
    /// Voicemod's background effects, on and off
    Background,
    GoToPage,
    /// Hides or shows the overlay while it shows always, else shows it for its time
    Overlay,
    /// Puts back the volumes the knob changed, as Voicemod has them
    ResetVolumes,
}

/// What a pad does
#[derive(Clone, Debug, PartialEq)]
pub enum Pad {
    Sound(Sound),
    /// Loads the voice into Voicemod's voice changer
    Voice(Voice),
    Action {
        action: Action,
        name: String,
        /// The page a GoToPage action opens, by its `Page::soundboard_id`
        page: Option<String>,
    },
    /// Runs one of the user's macros, by its id
    Macro {
        id: String,
        name: String,
    },
}

impl Pad {
    pub fn name(&self) -> &str {
        match self {
            Self::Sound(sound) => &sound.name,
            Self::Voice(voice) => &voice.name,
            Self::Action { name, .. } | Self::Macro { name, .. } => name,
        }
    }

    /// Within the user's licence
    pub fn enabled(&self) -> bool {
        match self {
            Self::Sound(sound) => sound.enabled,
            Self::Voice(voice) => voice.enabled,
            Self::Action { .. } | Self::Macro { .. } => true,
        }
    }

    pub fn image_url(&self) -> &str {
        match self {
            Self::Sound(sound) => &sound.image_url,
            Self::Voice(voice) => &voice.image_url,
            Self::Action { .. } | Self::Macro { .. } => "",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Page {
    /// The soundboard's id, FAVOURITES_ID, VOICES_ID or a page of the user's own
    pub soundboard_id: String,
    pub name: String,
    /// Which of the soundboard's pages this is, from 1, when it needs more than one
    pub part: usize,
    pub parts: usize,
    /// One entry per pad
    pub pads: Vec<Option<Pad>>,
    /// The LEDs' colours at rest the user picked, on their own pages
    pub colours: Vec<Option<[u8; 3]>>,
    /// What a pad does when pressed again, where the user picked it for that pad
    pub presses: Vec<Option<PressAgain>>,
    /// A sound's volume when started from that pad, in percent, where the user picked it
    pub volumes: Vec<Option<u8>>,
}

impl Page {
    pub fn colour(&self, pad: u8) -> Option<[u8; 3]> {
        self.colours.get(usize::from(pad)).copied().flatten()
    }

    pub fn press_again(&self, pad: u8) -> Option<PressAgain> {
        self.presses.get(usize::from(pad)).copied().flatten()
    }

    pub fn volume(&self, pad: u8) -> Option<u8> {
        self.volumes.get(usize::from(pad)).copied().flatten()
    }
}

/// The user's own pages first, then the favourite sounds, the voices in the licence and every soundboard in Voicemod's
/// order, 36 pads per page, each of Voicemod's as the user chose to show them.
pub fn pages(
    soundboards: &[Soundboard],
    voices: &[Voice],
    custom: &[CustomPage],
    macros: &[Macro],
    shown: &ShownPages,
    texts: &Texts,
) -> Vec<Page> {
    let voicemods = voicemod_pages(soundboards, voices, shown);
    // A pad that opens a page names it as the display does, if the board has it
    let page_name = |id: &str| -> Option<String> {
        if let Some(page) = custom.iter().find(|page| page.id == id) {
            return Some(page.name.clone());
        }
        let page = voicemods.iter().find(|page| page.soundboard_id == id)?;
        let name = match id {
            FAVOURITES_ID => texts.favourites,
            VOICES_ID => texts.voices,
            _ => &page.name,
        };
        Some(name.to_string())
    };
    let mut pages: Vec<Page> = custom
        .iter()
        .map(|page| custom_page(page, soundboards, voices, macros, &page_name, texts))
        .collect();
    pages.extend(voicemods);
    pages
}

/// The favourite sounds, the voices in the licence, then every soundboard in Voicemod's order.
fn voicemod_pages(soundboards: &[Soundboard], voices: &[Voice], shown: &ShownPages) -> Vec<Page> {
    let mut pages = Vec::new();
    let mut seen = HashSet::new();
    let favourites: Vec<Pad> = soundboards
        .iter()
        .flat_map(|soundboard| &soundboard.sounds)
        .filter(|sound| sound.favorited && seen.insert(&sound.id))
        .map(|sound| Pad::Sound(sound.clone()))
        .collect();

    if shown.favourites && !favourites.is_empty() {
        add_pages(&mut pages, FAVOURITES_ID, FAVOURITES_NAME, favourites);
    }
    // Only those that can be used: a free licence would fill pages with locked voices. The favourites first.
    let mut usable: Vec<&Voice> = voices.iter().filter(|voice| voice.enabled).collect();
    usable.sort_by_key(|voice| !voice.favorited);
    if shown.voices && !usable.is_empty() {
        let pads = usable.into_iter().map(|voice| Pad::Voice(voice.clone())).collect();
        add_pages(&mut pages, VOICES_ID, VOICES_NAME, pads);
    }
    for soundboard in soundboards.iter().filter(|soundboard| shown.shows(soundboard)) {
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
            colours: Vec::new(),
            presses: Vec::new(),
            volumes: Vec::new(),
        });
    }
}

/// A page of the user's own, its sounds and voices as Voicemod has them now. One Voicemod no longer has, or has not
/// sent yet, leaves its pad empty.
fn custom_page(
    page: &CustomPage,
    soundboards: &[Soundboard],
    voices: &[Voice],
    macros: &[Macro],
    page_name: &dyn Fn(&str) -> Option<String>,
    texts: &Texts,
) -> Page {
    let named = |name: &str, label: &str| if label.is_empty() { name } else { label }.to_string();
    let pad = |pad: &CustomPad| match &pad.content {
        Content::Sound { id, .. } => {
            let sound = soundboards
                .iter()
                .flat_map(|soundboard| &soundboard.sounds)
                .find(|sound| sound.id == *id)?;
            Some(Pad::Sound(Sound {
                name: named(&sound.name, &pad.label),
                ..sound.clone()
            }))
        }
        Content::Voice { id, .. } => {
            let voice = voices.iter().find(|voice| voice.id == *id)?;
            Some(Pad::Voice(Voice {
                name: named(&voice.name, &pad.label),
                ..voice.clone()
            }))
        }
        Content::Action { action, page } => {
            let target = page
                .as_deref()
                .filter(|_| *action == Action::GoToPage)
                .and_then(page_name);
            let own = match target {
                Some(target) => format!("{} {target}", texts.go_to),
                None => texts.action_name(*action).to_string(),
            };
            Some(Pad::Action {
                action: *action,
                name: named(&own, &pad.label),
                page: page.clone(),
            })
        }
        Content::Macro { id } => {
            let item = macros.iter().find(|item| item.id == *id)?;
            Some(Pad::Macro {
                id: id.clone(),
                name: named(&item.name, &pad.label),
            })
        }
    };
    let mut pads: Vec<Option<Pad>> = page.pads.iter().map(|slot| slot.as_ref().and_then(pad)).collect();
    pads.resize(PADS, None);
    let mut colours: Vec<Option<[u8; 3]>> = page.pads.iter().map(|slot| slot.as_ref()?.colour).collect();
    colours.resize(PADS, None);
    let mut presses: Vec<Option<PressAgain>> = page.pads.iter().map(|slot| slot.as_ref()?.press_again).collect();
    presses.resize(PADS, None);
    let mut volumes: Vec<Option<u8>> = page.pads.iter().map(|slot| slot.as_ref()?.volume).collect();
    volumes.resize(PADS, None);
    Page {
        soundboard_id: page.id.clone(),
        name: page.name.clone(),
        part: 1,
        parts: 1,
        pads,
        colours,
        presses,
        volumes,
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::display::texts::{ENGLISH, ITALIAN};

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
            image_url: String::new(),
        }
    }

    pub fn soundboard(id: &str, sounds: Vec<Sound>) -> Soundboard {
        Soundboard {
            id: id.to_string(),
            name: id.to_uppercase(),
            custom: false,
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
        let pages = pages(
            &[soundboard("big", big), soundboard("empty", vec![])],
            &[],
            &[],
            &[],
            &ShownPages::default(),
            &ENGLISH,
        );

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
    fn shows_only_the_pages_the_user_chose() {
        let favourite = Sound {
            favorited: true,
            ..sound("fav")
        };
        let mine = Soundboard {
            custom: true,
            ..soundboard("mine", vec![sound("y")])
        };
        let soundboards = [
            soundboard("a", vec![favourite]),
            soundboard("b", vec![sound("x")]),
            mine,
        ];
        let ids = |shown: ShownPages| -> Vec<String> {
            pages(&soundboards, &[voice("dark")], &[], &[], &shown, &ENGLISH)
                .into_iter()
                .map(|page| page.soundboard_id)
                .collect()
        };
        assert_eq!(ids(ShownPages::default()), [FAVOURITES_ID, VOICES_ID, "a", "b", "mine"]);
        let only_mine = ShownPages {
            only_mine: true,
            voices: false,
            ..ShownPages::default()
        };
        assert_eq!(
            ids(only_mine),
            [FAVOURITES_ID, "mine"],
            "the favourites keep their own switch"
        );
        let picked = ShownPages {
            favourites: false,
            hidden: vec!["b".into(), "mine".into()],
            ..ShownPages::default()
        };
        assert_eq!(ids(picked), [VOICES_ID, "a"]);
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
            &[],
            &[],
            &ShownPages::default(),
            &ENGLISH,
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
            image_url: String::new(),
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
        let pages = pages(
            &[soundboard("a", vec![favourite])],
            &voices,
            &[],
            &[],
            &ShownPages::default(),
            &ENGLISH,
        );

        assert_eq!(pages[1].soundboard_id, VOICES_ID);
        let names: Vec<_> = pages[1].pads.iter().flatten().map(Pad::name).collect();
        assert_eq!(names, ["V3", "Banana"]);
        assert_eq!(pages[2].soundboard_id, "a");
    }

    pub fn custom_pad(content: Content) -> Option<CustomPad> {
        Some(CustomPad {
            content,
            label: String::new(),
            colour: None,
            press_again: None,
            volume: None,
        })
    }

    pub fn action(action: Action) -> Content {
        Content::Action { action, page: None }
    }

    #[test]
    fn puts_the_users_own_pages_first() {
        let own = CustomPage {
            id: "page-1".into(),
            name: "Gaming".into(),
            pads: vec![
                Some(CustomPad {
                    label: "Fail".into(),
                    colour: Some([255, 140, 40]),
                    ..custom_pad(Content::Sound {
                        id: "honk".into(),
                        name: "Honk".into(),
                    })
                    .unwrap()
                }),
                custom_pad(Content::Sound {
                    id: "gone".into(),
                    name: "Gone".into(),
                }),
                custom_pad(Content::Voice {
                    id: "v1".into(),
                    name: String::new(),
                }),
                custom_pad(action(Action::MuteMic)),
                custom_pad(Content::Action {
                    action: Action::GoToPage,
                    page: Some(VOICES_ID.into()),
                }),
            ],
        };
        let pages = pages(
            &[soundboard("a", vec![sound("honk")])],
            &[voice("v1")],
            &[own],
            &[],
            &ShownPages::default(),
            &ITALIAN,
        );

        assert_eq!(
            (pages[0].soundboard_id.as_str(), pages[0].name.as_str()),
            ("page-1", "Gaming")
        );
        assert_eq!(pages[0].pads.len(), PADS);
        let names: Vec<_> = pages[0].pads.iter().map(|pad| pad.as_ref().map(Pad::name)).collect();
        assert_eq!(
            &names[..6],
            [
                Some("Fail"),
                None,
                Some("V1"),
                Some("Muto mic"),
                Some("Vai a Voci"),
                None
            ]
        );
        assert!(matches!(&pages[0].pads[0], Some(Pad::Sound(sound)) if sound.id == "honk"));
        assert_eq!(pages[0].colour(0), Some([255, 140, 40]));
        assert_eq!(pages[0].colour(3), None);
        assert_eq!(pages[1].soundboard_id, VOICES_ID);
    }
}
