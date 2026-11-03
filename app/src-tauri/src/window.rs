//! The settings window. Built on demand and kept hidden until its page has drawn a complete frame, so it never flashes;
//! a closed window stays ready for a quick reopen, then is destroyed to give its memory back.

use std::sync::{Mutex, MutexGuard};
use std::time::Duration;
use tauri::webview::PageLoadEvent;
use tauri::window::Color;
use tauri::{AppHandle, Emitter, Manager, Theme, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Window, WindowEvent};

/// Each window gets a new label ("main-1", "main-2"…), so a new one never clashes with one being destroyed
const LABEL_PREFIX: &str = "main-";
const BACKGROUND: Color = Color(0x0e, 0x0e, 0x10, 0xff); // --bg in src/styles.css
const SHOW_FALLBACK: Duration = Duration::from_secs(3);
const KEEP_ALIVE: Duration = Duration::from_secs(5 * 60);

#[derive(Default)]
pub struct WindowState(Mutex<State>);

#[derive(Default)]
struct State {
    phase: Phase,
    generation: u64,
    /// Bumped at every opening: a pending destroy only applies to the closing that scheduled it
    openings: u64,
}

impl State {
    fn label(&self) -> String {
        format!("{LABEL_PREFIX}{}", self.generation)
    }
}

#[derive(Default, PartialEq)]
enum Phase {
    #[default]
    Absent,
    /// Built and hidden, waiting for the page's first frame
    Loading,
    /// Drawn: shown, or hidden after a close
    Ready,
}

// Window calls may wait for the main thread: they are made after releasing the state lock, never while holding it.

pub fn open(app: &AppHandle) {
    let mut state = window_state(app);
    state.openings += 1;
    if state.phase == Phase::Loading {
        return; // shown by window_ready, or the fallback
    }
    if state.phase == Phase::Ready
        && let Some(window) = app.get_webview_window(&state.label())
    {
        drop(state);
        show(&window);
        return;
    }

    state.generation += 1;
    state.phase = Phase::Loading;
    let label = state.label();
    drop(state);
    build(app.clone(), label);
}

fn build(app: AppHandle, label: String) {
    // Building a window from a menu or single-instance callback deadlocks on Windows: use another thread
    std::thread::spawn(move || {
        let builder = WebviewWindowBuilder::new(&app, &label, WebviewUrl::default())
            .title("Drezimo Soundboard")
            // Fits a 1366×768 screen with its taskbar
            .inner_size(880.0, 660.0)
            .min_inner_size(760.0, 520.0)
            .center()
            .visible(false)
            .background_color(BACKGROUND)
            // Dark whatever the system theme, title bar included
            .theme(Some(Theme::Dark))
            // Tauri's own file drops would take the page's drags, the pads moved in the Pages editor
            .disable_drag_drop_handler()
            // A reloaded page knows of no board test: one left running would keep the board from playing
            .on_page_load(|window, payload| {
                if payload.event() == PageLoadEvent::Started {
                    crate::device::end_board_test(window.app_handle());
                }
            });
        #[cfg(target_os = "macos")]
        let builder = builder.title_bar_style(tauri::TitleBarStyle::Transparent);

        if let Err(error) = builder.build() {
            crate::errors::log(format!("cannot open the window: {error}"));
            let mut state = window_state(&app);
            if state.label() == label {
                state.phase = Phase::Absent;
            }
            return;
        }
        // In case the page never reports its first frame
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(SHOW_FALLBACK).await;
            reveal(&app, &label);
        });
    });
}

/// Called by the page once its first complete frame is drawn.
#[tauri::command]
pub fn window_ready(window: WebviewWindow) {
    reveal(window.app_handle(), window.label());
}

/// Shows the window `label` if it is the one waiting for its first frame.
fn reveal(app: &AppHandle, label: &str) {
    {
        let mut state = window_state(app);
        if state.phase != Phase::Loading || state.label() != label {
            return;
        }
        state.phase = Phase::Ready;
    }
    if let Some(window) = app.get_webview_window(label) {
        show(&window);
    }
}

pub fn on_event(window: &Window, event: &WindowEvent) {
    let WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };
    if !window.label().starts_with(LABEL_PREFIX) {
        return;
    }
    api.prevent_close();
    let _ = window.hide();
    // The page stays: it learns it was closed, rather than only hidden or minimised
    let _ = window.emit("window-closed", ());
    // Nobody sees the test any more: the board plays again
    crate::device::end_board_test(window.app_handle());

    let app = window.app_handle().clone();
    let label = window.label().to_string();
    let opening = window_state(&app).openings;
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(KEEP_ALIVE).await;
        {
            let mut state = window_state(&app);
            if state.openings != opening {
                return;
            }
            state.phase = Phase::Absent;
        }
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.destroy();
        }
    });
}

/// Whether the window is on the screen, not minimised.
pub fn is_shown(app: &AppHandle) -> bool {
    let label = window_state(app).label();
    app.get_webview_window(&label)
        .is_some_and(|window| window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(false))
}

fn window_state(app: &AppHandle) -> MutexGuard<'_, State> {
    app.state::<WindowState>().inner().0.lock().unwrap()
}

fn show(window: &WebviewWindow) {
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}
