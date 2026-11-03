//! The page the board showed last, kept in the app's config folder so the next start shows it again.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const FILE_NAME: &str = "page.json";

#[derive(Deserialize, Serialize)]
struct Saved {
    /// A soundboard's id, or the Favourites', the Voices' or that of a page of the user's own
    soundboard: String,
    part: usize,
}

/// The page as soundboard and part, if one was kept.
pub fn load(app: &AppHandle) -> Option<(String, usize)> {
    read(&fs::read_to_string(path(app)?).ok()?)
}

/// A page that cannot be kept is only forgotten: the next start shows the first page of sounds.
pub fn save(app: &AppHandle, page: &(String, usize)) {
    if let Err(error) = write(app, page) {
        eprintln!("cannot keep the page shown: {error}");
    }
}

fn read(text: &str) -> Option<(String, usize)> {
    let saved: Saved = serde_json::from_str(text).ok()?;
    Some((saved.soundboard, saved.part))
}

fn text((soundboard, part): &(String, usize)) -> String {
    let saved = Saved {
        soundboard: soundboard.clone(),
        part: *part,
    };
    serde_json::to_string(&saved).unwrap_or_default()
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    Some(app.path().app_config_dir().ok()?.join(FILE_NAME))
}

fn write(app: &AppHandle, page: &(String, usize)) -> Result<(), String> {
    let path = path(app).ok_or("no config folder")?;
    let folder = path.parent().ok_or("no config folder")?;
    fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    // Written aside first, so a crash never leaves half a file
    let written = path.with_extension("json.new");
    fs::write(&written, text(page)).map_err(|error| error.to_string())?;
    fs::rename(&written, &path).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_it_wrote() {
        let page = ("b".to_string(), 2);
        assert_eq!(read(&text(&page)), Some(page));
    }

    #[test]
    fn an_unreadable_file_is_no_page() {
        assert_eq!(read(r#"{"part": 1}"#), None);
        assert_eq!(read(""), None);
    }
}
