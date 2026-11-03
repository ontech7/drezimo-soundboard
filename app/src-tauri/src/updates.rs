//! Updates of the app itself, for the builds published on GitHub (docs/project.md, section 8): looked for at start and
//! every few hours, but only while nobody uses the app, so the version found is installed at once. The app closes for
//! the install; on Windows the installer shows a small progress bar and opens the app again.

use crate::device::{LastUse, Status};
use crate::firmware::FirmwareUpdate;
use crate::news::{self, News};
use crate::window;
use serde::Serialize;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::UpdaterExt;

/// Built by the release workflow: only those builds update themselves
pub const OFFICIAL: bool = option_env!("DREZIMO_OFFICIAL").is_some();
const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);
/// After a failure, such as no network yet at login
const RETRY_AFTER: Duration = Duration::from_secs(10 * 60);
/// How long the board must have been left alone before the app closes to install an update
const IDLE_BEFORE_INSTALL: Duration = Duration::from_secs(10 * 60);
const IDLE_CHECK: Duration = Duration::from_secs(30);

/// For the window's General page. Times are Unix milliseconds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum UpdateState {
    /// A build made here: it does not update itself
    Local,
    /// For the first moment nobody uses the app
    Waiting,
    Checking,
    UpToDate {
        checked_at: i64,
    },
    Downloading {
        version: String,
    },
    /// Downloaded, installed the next moment nobody uses the app
    Ready {
        version: String,
    },
    /// Tried again after `RETRY_AFTER`
    Failed {
        at: i64,
    },
}

pub struct UpdateStateHolder(Mutex<UpdateState>);

impl Default for UpdateStateHolder {
    fn default() -> Self {
        Self(Mutex::new(if OFFICIAL {
            UpdateState::Waiting
        } else {
            UpdateState::Local
        }))
    }
}

#[tauri::command]
pub fn update_state(state: State<UpdateStateHolder>) -> UpdateState {
    state.0.lock().unwrap().clone()
}

fn set_state(app: &AppHandle, state: UpdateState) {
    *app.state::<UpdateStateHolder>().0.lock().unwrap() = state.clone();
    let _ = app.emit("update-state", state);
}

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn start(app: &AppHandle) {
    if let Err(error) = app.plugin(tauri_plugin_updater::Builder::new().build()) {
        crate::errors::log(format!("cannot look for updates: {error}"));
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let wait = match update(&app).await {
                Ok(()) => CHECK_EVERY,
                Err(error) => {
                    crate::errors::log(format!("cannot update the app: {error}"));
                    set_state(&app, UpdateState::Failed { at: now() });
                    RETRY_AFTER
                }
            };
            tokio::time::sleep(wait).await;
        }
    });
}

/// Between two checks, the window keeps showing how the last one went.
async fn update(app: &AppHandle) -> tauri_plugin_updater::Result<()> {
    until_idle(app).await;
    set_state(app, UpdateState::Checking);
    let Some(update) = app.updater()?.check().await? else {
        set_state(app, UpdateState::UpToDate { checked_at: now() });
        return Ok(());
    };
    let version = update.version.clone();
    eprintln!("downloading version {version} of the app");
    set_state(
        app,
        UpdateState::Downloading {
            version: version.clone(),
        },
    );
    let package = update.download(|_, _| {}, || {}).await?;
    set_state(
        app,
        UpdateState::Ready {
            version: version.clone(),
        },
    );
    // In case the app was used during the download
    until_idle(app).await;
    let notes = update.body.as_deref().and_then(news::notes);
    news::save(app, &News { version, notes });
    update.install(package)?;
    // Windows has already closed the app to run the installer
    app.restart()
}

async fn until_idle(app: &AppHandle) {
    while !idle(app) {
        tokio::time::sleep(IDLE_CHECK).await;
    }
}

/// True while the board has not been used for a while, or not at all since the app started, its firmware is not being
/// written, and the window is closed.
fn idle(app: &AppHandle) -> bool {
    let board_updating = app
        .state::<Mutex<Status>>()
        .lock()
        .unwrap()
        .firmware_update
        .as_ref()
        .is_some_and(FirmwareUpdate::in_progress);
    let last_use = *app.state::<LastUse>().0.lock().unwrap();
    let board_left = last_use.is_none_or(|at| at.elapsed() >= IDLE_BEFORE_INSTALL);
    board_left && !board_updating && !window::is_shown(app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_reads_the_state_in_camel_case() {
        let json = |state: UpdateState| serde_json::to_value(state).unwrap();
        assert_eq!(json(UpdateState::Local), serde_json::json!({ "state": "local" }));
        assert_eq!(
            json(UpdateState::UpToDate { checked_at: 5 }),
            serde_json::json!({ "state": "upToDate", "checkedAt": 5 })
        );
    }
}
