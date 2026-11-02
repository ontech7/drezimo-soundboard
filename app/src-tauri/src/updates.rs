//! Updates of the app itself, for the builds published on GitHub (docs/project.md, section 8): looked for at start and
//! every few hours, but only while nobody uses the app, so the version found is installed at once. The app closes for
//! the install; on Windows the installer shows a small progress bar and opens the app again.

use crate::device::{LastUse, Status};
use crate::firmware::FirmwareUpdate;
use crate::window;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::UpdaterExt;

/// Built by the release workflow: only those builds update themselves
pub const OFFICIAL: bool = option_env!("DREZIMO_OFFICIAL").is_some();
const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);
/// After a failure, such as no network yet at login
const RETRY_AFTER: Duration = Duration::from_secs(10 * 60);
/// How long the board must have been left alone before the app closes to install an update
const IDLE_BEFORE_INSTALL: Duration = Duration::from_secs(10 * 60);
const IDLE_CHECK: Duration = Duration::from_secs(30);

pub fn start(app: &AppHandle) {
    if let Err(error) = app.plugin(tauri_plugin_updater::Builder::new().build()) {
        eprintln!("cannot look for updates: {error}");
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let wait = match update(&app).await {
                Ok(()) => CHECK_EVERY,
                Err(error) => {
                    eprintln!("cannot update the app: {error}");
                    RETRY_AFTER
                }
            };
            tokio::time::sleep(wait).await;
        }
    });
}

async fn update(app: &AppHandle) -> tauri_plugin_updater::Result<()> {
    until_idle(app).await;
    let Some(update) = app.updater()?.check().await? else {
        return Ok(());
    };
    eprintln!("downloading version {} of the app", update.version);
    let package = update.download(|_, _| {}, || {}).await?;
    // In case the app was used during the download
    until_idle(app).await;
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
