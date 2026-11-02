//! The user's own pages of 36 pads, made in the window's Pages editor and kept in a JSON file in the app's config
//! folder. They come first on the board, before Voicemod's soundboards (docs/project.md, section 8).

use crate::pictures;
use crate::protocol::PAD_COUNT;
use crate::soundboard::{Action, Pad, Soundboard, Voice};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

const FILE_NAME: &str = "pages.json";
/// Before every id the editor makes, so a page never takes a soundboard's id, nor the Favourites' or the Voices'
const ID_PREFIX: &str = "page-";
const PADS: usize = PAD_COUNT as usize;
const MAX_PAGES: usize = 50;
/// In characters, as the editor allows
const MAX_NAME: usize = 24;
const MAX_LABEL: usize = 20;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomPage {
    /// Made by the editor, never one of Voicemod's soundboard ids
    pub id: String,
    pub name: String,
    /// One per pad, from the top left; null for an empty pad
    #[serde(deserialize_with = "pads_known")]
    pub pads: Vec<Option<CustomPad>>,
}

/// A pad this app does not know, from another version, is left empty rather than losing the page.
fn pads_known<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Option<CustomPad>>, D::Error> {
    let pads = Vec::<Value>::deserialize(deserializer)?;
    Ok(pads.into_iter().map(|pad| CustomPad::deserialize(pad).ok()).collect())
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomPad {
    #[serde(flatten)]
    pub content: Content,
    /// Shown instead of its own name, when not empty
    #[serde(default)]
    pub label: String,
    /// The LED's colour at rest, instead of its kind's
    #[serde(default)]
    pub colour: Option<[u8; 3]>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Content {
    /// By Voicemod's id, with its name when picked, for the editor to show while Voicemod is away or once it is gone
    Sound {
        id: String,
        #[serde(default)]
        name: String,
    },
    Voice {
        id: String,
        #[serde(default)]
        name: String,
    },
    Action {
        action: Action,
        /// The page a GoToPage action opens, by its id
        #[serde(default, skip_serializing_if = "Option::is_none")]
        page: Option<String>,
    },
    /// One of the user's macros, by its id
    Macro { id: String },
}

pub struct CustomPagesState(Mutex<Vec<CustomPage>>);

impl CustomPagesState {
    pub fn get(&self) -> Vec<CustomPage> {
        self.0.lock().unwrap().clone()
    }
}

/// What the editor picks from: Voicemod's sounds and voices as it last sent them, kept while Voicemod is away
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    soundboards: Vec<LibrarySoundboard>,
    voices: Vec<LibraryItem>,
    /// Whether the board has a Favourites page
    favourites: bool,
    /// The sounds' lengths known so far, in seconds
    #[serde(skip)]
    lengths: HashMap<String, f32>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct LibrarySoundboard {
    id: String,
    name: String,
    sounds: Vec<LibraryItem>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct LibraryItem {
    id: String,
    name: String,
    /// Within the user's licence
    enabled: bool,
    /// On the pad-image scheme (src/pictures.rs)
    picture: Option<String>,
    /// A sound's, in seconds, once known
    length: Option<f32>,
}

impl Library {
    pub fn set_soundboards(&mut self, soundboards: &[Soundboard]) {
        self.favourites = soundboards
            .iter()
            .any(|soundboard| soundboard.sounds.iter().any(|sound| sound.favorited));
        self.soundboards = soundboards
            .iter()
            .map(|soundboard| LibrarySoundboard {
                id: soundboard.id.clone(),
                name: soundboard.name.clone(),
                sounds: soundboard
                    .sounds
                    .iter()
                    .map(|sound| LibraryItem {
                        length: self.lengths.get(&sound.id).copied(),
                        ..item(Pad::Sound(sound.clone()))
                    })
                    .collect(),
            })
            .collect();
    }

    pub fn set_lengths(&mut self, lengths: &HashMap<String, Duration>) {
        self.lengths = lengths
            .iter()
            .map(|(id, length)| (id.clone(), length.as_secs_f32()))
            .collect();
        for sound in self
            .soundboards
            .iter_mut()
            .flat_map(|soundboard| &mut soundboard.sounds)
        {
            sound.length = self.lengths.get(&sound.id).copied();
        }
    }

    pub fn set_voices(&mut self, voices: &[Voice]) {
        self.voices = voices.iter().map(|voice| item(Pad::Voice(voice.clone()))).collect();
    }
}

fn item(pad: Pad) -> LibraryItem {
    let (id, name) = match &pad {
        Pad::Sound(sound) => (sound.id.clone(), sound.name.clone()),
        Pad::Voice(voice) => (voice.id.clone(), voice.name.clone()),
        Pad::Action { name, .. } | Pad::Macro { name, .. } => (String::new(), name.clone()),
    };
    LibraryItem {
        id,
        name,
        enabled: pad.enabled(),
        picture: pictures::path(&pad),
        length: None,
    }
}

#[derive(Default)]
pub struct LibraryState(pub Mutex<Library>);

/// Within what the editor offers: unique ids of its own, at most 50 pages of 36 pads, short names.
pub fn checked(pages: Vec<CustomPage>) -> Vec<CustomPage> {
    let mut ids = HashSet::new();
    pages
        .into_iter()
        .filter(|page| page.id.starts_with(ID_PREFIX) && page.id.len() <= 64 && ids.insert(page.id.clone()))
        .take(MAX_PAGES)
        .enumerate()
        .map(|(index, mut page)| {
            page.name = shortened(&page.name, MAX_NAME);
            if page.name.is_empty() {
                page.name = format!("#{}", index + 1);
            }
            page.pads.resize(PADS, None);
            for pad in page.pads.iter_mut().flatten() {
                pad.label = shortened(&pad.label, MAX_LABEL);
            }
            page
        })
        .collect()
}

fn shortened(text: &str, max: usize) -> String {
    text.trim().chars().take(max).collect()
}

/// The saved pages, or none.
pub fn load(app: &AppHandle) -> CustomPagesState {
    let saved = path(app).and_then(|path| fs::read_to_string(path).ok());
    let pages = match saved.map(|text| serde_json::from_str::<Vec<CustomPage>>(&text)) {
        Some(Ok(pages)) => checked(pages),
        Some(Err(error)) => {
            // Kept aside rather than written over at the next save
            eprintln!("unreadable pages, starting without: {error}");
            if let Some(path) = path(app) {
                let _ = fs::rename(&path, path.with_extension("json.unreadable"));
            }
            Vec::new()
        }
        None => Vec::new(),
    };
    CustomPagesState(Mutex::new(pages))
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    Some(app.path().app_config_dir().ok()?.join(FILE_NAME))
}

fn save(app: &AppHandle, pages: &[CustomPage]) -> Result<(), String> {
    let path = path(app).ok_or("no config folder")?;
    let folder = path.parent().ok_or("no config folder")?;
    fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    let text = serde_json::to_string_pretty(pages).map_err(|error| error.to_string())?;
    // Written aside first, so a crash never leaves half a file
    let written = path.with_extension("json.new");
    fs::write(&written, text).map_err(|error| error.to_string())?;
    fs::rename(&written, &path).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn custom_pages(state: State<CustomPagesState>) -> Vec<CustomPage> {
    state.get()
}

#[tauri::command]
pub fn set_custom_pages(app: AppHandle, pages: Vec<CustomPage>) -> Result<Vec<CustomPage>, String> {
    let pages = checked(pages);
    app.state::<CustomPagesState>().0.lock().unwrap().clone_from(&pages);
    crate::device::apply_custom_pages(&app, pages.clone());
    let _ = app.emit("custom-pages", &pages);
    save(&app, &pages)?;
    Ok(pages)
}

#[tauri::command]
pub fn library(state: State<LibraryState>) -> Library {
    state.0.lock().unwrap().clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_writes_the_pages_file() {
        let json = r#"[{"id": "page-1", "name": "Gaming", "pads": [
            {"kind": "sound", "id": "s1", "name": "Honk", "label": "Fail", "colour": [255, 140, 40]},
            null,
            {"kind": "voice", "id": "v1"},
            {"kind": "action", "action": "muteMic"},
            {"kind": "action", "action": "goToPage", "page": "page-2"},
            {"kind": "action", "action": "somethingNew"},
            {"kind": "macro", "steps": []}]}]"#;
        let pages: Vec<CustomPage> = serde_json::from_str(json).unwrap();
        let pads = &pages[0].pads;
        assert_eq!(
            pads[0],
            Some(CustomPad {
                content: Content::Sound {
                    id: "s1".into(),
                    name: "Honk".into()
                },
                label: "Fail".into(),
                colour: Some([255, 140, 40]),
            })
        );
        assert_eq!(pads[1], None);
        assert!(matches!(&pads[2], Some(CustomPad { content: Content::Voice { id, .. }, .. }) if id == "v1"));
        assert!(matches!(
            &pads[4],
            Some(CustomPad { content: Content::Action { action: Action::GoToPage, page: Some(page) }, .. }) if page == "page-2"
        ));
        assert_eq!(pads[5..], [None, None], "pads of another version stay empty");

        let written = serde_json::to_string(&pages).unwrap();
        assert_eq!(serde_json::from_str::<Vec<CustomPage>>(&written).unwrap(), pages);
    }

    #[test]
    fn keeps_the_pages_within_what_the_editor_offers() {
        let page = |id: &str, name: &str| CustomPage {
            id: id.into(),
            name: name.into(),
            pads: vec![Some(CustomPad {
                content: Content::Action {
                    action: Action::MuteMic,
                    page: None,
                },
                label: "  a label far too long for the display  ".into(),
                colour: None,
            })],
        };
        let pages = checked(vec![
            page("page-a", "  Gaming  "),
            page("page-a", "Again"),
            page("favourites", "Not an id of its own"),
            page("page-b", " "),
        ]);

        let names: Vec<_> = pages.iter().map(|page| page.name.as_str()).collect();
        assert_eq!(names, ["Gaming", "#2"]);
        assert_eq!(pages[0].pads.len(), PADS);
        assert_eq!(pages[0].pads[0].as_ref().unwrap().label, "a label far too long");
    }
}
