//! What is new after the app updated itself: the release's notes, kept across the install in the config folder and shown
//! once in the window.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

const FILE_NAME: &str = "news.json";
/// What follows it in a release's text is for the GitHub page only: how to install
const SEPARATOR: &str = "---";

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct News {
    pub version: String,
    pub notes: Option<String>,
}

pub struct NewsState(Mutex<Option<News>>);

/// The news of the version running, if it was just installed. Older news is forgotten.
pub fn load(app: &AppHandle) -> NewsState {
    let saved = path(app)
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str::<News>(&text).ok());
    let running = app.package_info().version.to_string();
    let news = saved.and_then(|news| kept(news, &running));
    if news.is_none() {
        forget(app);
    }
    NewsState(Mutex::new(news))
}

/// Kept for the next start, right before the update installs.
pub fn save(app: &AppHandle, news: &News) {
    if let Err(error) = write(app, news) {
        crate::errors::log(format!("cannot keep what is new: {error}"));
    }
}

/// The release's own words, without what follows the separator.
pub fn notes(body: &str) -> Option<String> {
    let lines: Vec<&str> = body.lines().take_while(|line| line.trim() != SEPARATOR).collect();
    let notes = lines.join("\n").trim().to_string();
    (!notes.is_empty()).then_some(notes)
}

fn kept(news: News, running: &str) -> Option<News> {
    (news.version == running).then_some(news)
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    Some(app.path().app_config_dir().ok()?.join(FILE_NAME))
}

fn write(app: &AppHandle, news: &News) -> Result<(), String> {
    let path = path(app).ok_or("no config folder")?;
    let folder = path.parent().ok_or("no config folder")?;
    fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    let text = serde_json::to_string(news).map_err(|error| error.to_string())?;
    // Written aside first, so a crash never leaves half a file
    let written = path.with_extension("json.new");
    fs::write(&written, text).map_err(|error| error.to_string())?;
    fs::rename(&written, &path).map_err(|error| error.to_string())
}

fn forget(app: &AppHandle) {
    if let Some(path) = path(app).filter(|path| path.exists())
        && let Err(error) = fs::remove_file(path)
    {
        crate::errors::log(format!("cannot forget what was new: {error}"));
    }
}

#[tauri::command]
pub fn news(state: State<NewsState>) -> Option<News> {
    state.0.lock().unwrap().clone()
}

#[tauri::command]
pub fn dismiss_news(app: AppHandle) {
    *app.state::<NewsState>().0.lock().unwrap() = None;
    forget(&app);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_stop_at_the_separator() {
        assert_eq!(
            notes("Nuove macro\n\n---\nThe Windows installer is the `-setup.exe` file."),
            Some("Nuove macro".to_string())
        );
        assert_eq!(notes("---\nThe Windows installer is the `-setup.exe` file."), None);
        assert_eq!(notes(""), None);
    }

    #[test]
    fn news_of_another_version_is_dropped() {
        let news = |version: &str| News {
            version: version.into(),
            notes: None,
        };
        assert_eq!(kept(news("0.2.0"), "0.1.0"), None);
        assert_eq!(kept(news("0.2.0"), "0.2.0"), Some(news("0.2.0")));
    }
}
