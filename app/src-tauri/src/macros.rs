//! The user's macros, made in the window's Macro section and kept in a JSON file in the app's config folder: sequences
//! of steps a pad of their own pages runs (docs/project.md, section 8). `controller/macros.rs` runs them.

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

const FILE_NAME: &str = "macros.json";
/// Before every id the editor makes
const ID_PREFIX: &str = "macro-";
const MAX_MACROS: usize = 100;
const MAX_STEPS: usize = 50;
/// In characters, as the editor allows
const MAX_NAME: usize = 24;
const MIN_WAIT: u32 = 100;
const MAX_WAIT: u32 = 60_000;
const MAX_REPEAT: u8 = 3;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Macro {
    pub id: String,
    pub name: String,
    /// The LED's colour at rest on its pads, unless a pad has its own
    #[serde(default)]
    pub colour: Option<[u8; 3]>,
    /// What a press does while it runs
    #[serde(default)]
    pub again: Again,
    /// How many times the steps run; 0 for until it is stopped
    #[serde(default = "once")]
    pub repeat: u8,
    /// At the end, the voice and the switches it changed go back as they were
    #[serde(default)]
    pub restore: bool,
    #[serde(deserialize_with = "steps_known")]
    pub steps: Vec<Step>,
}

fn once() -> u8 {
    1
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Again {
    Restart,
    /// Also for what another version may have saved
    #[default]
    #[serde(other)]
    Stop,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Step {
    /// By Voicemod's id, with its name when picked, for the editor to show while Voicemod is away
    Sound {
        id: String,
        #[serde(default)]
        name: String,
        /// The next step waits for the sound's end
        #[serde(default, rename = "waitEnd")]
        wait_end: bool,
    },
    Wait {
        millis: u32,
    },
    Voice {
        id: String,
        #[serde(default)]
        name: String,
    },
    /// One of the voices in the licence, other than the current one
    RandomVoice,
    /// The voice loaded when the macro started
    PreviousVoice,
    Switch {
        switch: Switch,
        state: SwitchState,
    },
    StopSounds,
    GoToPage {
        page: String,
    },
}

/// Voicemod's switches a macro can set
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Switch {
    MuteMic,
    VoiceChanger,
    HearMyself,
    MuteForMe,
    Background,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SwitchState {
    On,
    Off,
    Toggle,
}

/// A step this app does not know, from another version, is left out rather than losing the macro.
fn steps_known<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Step>, D::Error> {
    let steps = Vec::<Value>::deserialize(deserializer)?;
    Ok(steps
        .into_iter()
        .filter_map(|step| Step::deserialize(step).ok())
        .collect())
}

pub struct MacrosState(Mutex<Vec<Macro>>);

impl MacrosState {
    pub fn get(&self) -> Vec<Macro> {
        self.0.lock().unwrap().clone()
    }
}

/// Within what the editor offers: unique ids of its own, short names, at most 50 steps, waits of 0.1 to 60 s.
pub fn checked(macros: Vec<Macro>) -> Vec<Macro> {
    let mut ids = HashSet::new();
    macros
        .into_iter()
        .filter(|item| item.id.starts_with(ID_PREFIX) && item.id.len() <= 64 && ids.insert(item.id.clone()))
        .take(MAX_MACROS)
        .enumerate()
        .map(|(index, mut item)| {
            item.name = item.name.trim().chars().take(MAX_NAME).collect();
            if item.name.is_empty() {
                item.name = format!("#{}", index + 1);
            }
            item.repeat = item.repeat.min(MAX_REPEAT);
            item.steps.truncate(MAX_STEPS);
            for step in &mut item.steps {
                if let Step::Wait { millis } = step {
                    *millis = (*millis).clamp(MIN_WAIT, MAX_WAIT);
                }
            }
            item
        })
        .collect()
}

/// The saved macros, or none. A macro this app cannot read, from another version, is left out alone.
pub fn load(app: &AppHandle) -> MacrosState {
    let saved = path(app).and_then(|path| fs::read_to_string(path).ok());
    let macros = match saved.map(|text| serde_json::from_str::<Vec<Value>>(&text)) {
        Some(Ok(macros)) => checked(
            macros
                .into_iter()
                .filter_map(|item| Macro::deserialize(item).ok())
                .collect(),
        ),
        Some(Err(error)) => {
            // Kept aside rather than written over at the next save
            crate::errors::log(format!("unreadable macros, starting without: {error}"));
            if let Some(path) = path(app) {
                let _ = fs::rename(&path, path.with_extension("json.unreadable"));
            }
            Vec::new()
        }
        None => Vec::new(),
    };
    MacrosState(Mutex::new(macros))
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    Some(app.path().app_config_dir().ok()?.join(FILE_NAME))
}

fn save(app: &AppHandle, macros: &[Macro]) -> Result<(), String> {
    let path = path(app).ok_or("no config folder")?;
    let folder = path.parent().ok_or("no config folder")?;
    fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    let text = serde_json::to_string_pretty(macros).map_err(|error| error.to_string())?;
    // Written aside first, so a crash never leaves half a file
    let written = path.with_extension("json.new");
    fs::write(&written, text).map_err(|error| error.to_string())?;
    fs::rename(&written, &path).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn macros(state: State<MacrosState>) -> Vec<Macro> {
    state.get()
}

/// Runs the macro as a press on one of its pads would, or stops it.
#[tauri::command]
pub fn try_macro(app: AppHandle, id: String) {
    crate::device::try_macro(&app, id);
}

/// Stops the macro running, if any.
#[tauri::command]
pub fn stop_macro(app: AppHandle) {
    crate::device::stop_macro(&app);
}

#[tauri::command]
pub fn set_macros(app: AppHandle, macros: Vec<Macro>) -> Result<Vec<Macro>, String> {
    let macros = checked(macros);
    app.state::<MacrosState>().0.lock().unwrap().clone_from(&macros);
    crate::device::apply_macros(&app, macros.clone());
    let _ = app.emit("macros", &macros);
    save(&app, &macros).inspect_err(|error| crate::errors::log(format!("cannot save the macros: {error}")))?;
    Ok(macros)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_writes_the_macros_file() {
        let json = r#"[{"id": "macro-1", "name": "Entrance", "colour": [255, 140, 0], "again": "restart", "repeat": 2,
            "restore": true, "steps": [
                {"type": "sound", "id": "s1", "name": "Airhorn", "waitEnd": true},
                {"type": "wait", "millis": 500},
                {"type": "voice", "id": "v1", "name": "Dark"},
                {"type": "randomVoice"},
                {"type": "previousVoice"},
                {"type": "switch", "switch": "muteMic", "state": "on"},
                {"type": "stopSounds"},
                {"type": "goToPage", "page": "page-1"},
                {"type": "somethingNew"}]}]"#;
        let macros: Vec<Macro> = serde_json::from_str(json).unwrap();
        let entrance = &macros[0];
        assert_eq!(
            (entrance.again, entrance.repeat, entrance.restore),
            (Again::Restart, 2, true)
        );
        assert_eq!(entrance.steps.len(), 8, "a step of another version is left out");
        assert_eq!(
            entrance.steps[0],
            Step::Sound {
                id: "s1".into(),
                name: "Airhorn".into(),
                wait_end: true
            }
        );
        assert_eq!(
            entrance.steps[5],
            Step::Switch {
                switch: Switch::MuteMic,
                state: SwitchState::On
            }
        );

        let written = serde_json::to_string(&macros).unwrap();
        assert_eq!(serde_json::from_str::<Vec<Macro>>(&written).unwrap(), macros);
    }

    #[test]
    fn keeps_the_macros_within_what_the_editor_offers() {
        let item = |id: &str, name: &str, millis: u32| Macro {
            id: id.into(),
            name: name.into(),
            colour: None,
            again: Again::Stop,
            repeat: 9,
            restore: false,
            steps: vec![Step::Wait { millis }],
        };
        let macros = checked(vec![
            item("macro-a", "  Entrance  ", 10),
            item("macro-a", "Again", 500),
            item("page-1", "Not a macro id", 500),
            item("macro-b", "", 90_000),
        ]);

        let names: Vec<_> = macros.iter().map(|item| item.name.as_str()).collect();
        assert_eq!(names, ["Entrance", "#2"]);
        assert_eq!(macros[0].repeat, MAX_REPEAT);
        assert_eq!(macros[0].steps, [Step::Wait { millis: MIN_WAIT }]);
        assert_eq!(macros[1].steps, [Step::Wait { millis: MAX_WAIT }]);
    }
}
