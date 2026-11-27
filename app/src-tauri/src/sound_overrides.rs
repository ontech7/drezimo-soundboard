//! The sounds whose options a macro turned off in Voicemod, as they were, kept in the app's config folder until put
//! back: an app closed halfway puts them back at its next start (src/controller/overrides.rs).

use crate::controller::OwnOptions;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const FILE_NAME: &str = "sound-overrides.json";

pub fn load(app: &AppHandle) -> HashMap<String, OwnOptions> {
    path(app)
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// None left, the file goes
pub fn save(app: &AppHandle, overrides: &HashMap<String, OwnOptions>) {
    if let Err(error) = write(app, overrides) {
        crate::errors::log(format!("cannot keep the sounds a macro changed: {error}"));
    }
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    Some(app.path().app_config_dir().ok()?.join(FILE_NAME))
}

fn write(app: &AppHandle, overrides: &HashMap<String, OwnOptions>) -> Result<(), String> {
    let path = path(app).ok_or("no config folder")?;
    if overrides.is_empty() {
        return match fs::remove_file(&path) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error.to_string()),
            _ => Ok(()),
        };
    }
    let folder = path.parent().ok_or("no config folder")?;
    fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    // Written aside first, so a crash never leaves half a file
    let written = path.with_extension("json.new");
    let text = serde_json::to_string(overrides).map_err(|error| error.to_string())?;
    fs::write(&written, text).map_err(|error| error.to_string())?;
    fs::rename(&written, &path).map_err(|error| error.to_string())
}
