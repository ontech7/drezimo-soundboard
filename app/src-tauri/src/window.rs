//! The settings window. Built on demand and kept hidden until its page has drawn a complete frame, so it never flashes;
//! a closed window stays ready for a quick reopen, then is destroyed to give its memory back.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::Duration;
use tauri::window::Color;
use tauri::{AppHandle, Manager, Theme, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Window, WindowEvent};

const LABEL: &str = "main";
const BACKGROUND: Color = Color(0x0e, 0x0e, 0x10, 0xff); // --bg in src/styles.css
const SHOW_FALLBACK: Duration = Duration::from_secs(3);
const KEEP_ALIVE: Duration = Duration::from_secs(5 * 60);

#[derive(Default)]
pub struct WindowState {
    /// Bumped at every opening: a pending destroy only applies to the closing that scheduled it
    openings: AtomicU64,
    /// Whether the current window has been shown since it was built
    shown: AtomicBool,
}

pub fn open(app: &AppHandle) {
    let state = app.state::<WindowState>();
    state.openings.fetch_add(1, Ordering::SeqCst);
    if let Some(window) = app.get_webview_window(LABEL) {
        show(&window);
        return;
    }

    state.shown.store(false, Ordering::SeqCst);
    // Building a window from a menu or single-instance callback deadlocks on Windows: use another thread
    let app = app.clone();
    thread::spawn(move || {
        let builder = WebviewWindowBuilder::new(&app, LABEL, WebviewUrl::default())
            .title("Drezimo Soundboard")
            .inner_size(480.0, 560.0)
            .min_inner_size(400.0, 440.0)
            .center()
            .visible(false)
            .background_color(BACKGROUND)
            // Dark whatever the system theme, title bar included
            .theme(Some(Theme::Dark));
        #[cfg(target_os = "macos")]
        let builder = builder.title_bar_style(tauri::TitleBarStyle::Transparent);
        let built = builder.build();
        if let Err(error) = built {
            eprintln!("cannot open the window: {error}");
            return;
        }
        // In case the page never reports its first frame
        thread::sleep(SHOW_FALLBACK);
        window_ready(app);
    });
}

/// Called by the page once its first complete frame is drawn.
#[tauri::command]
pub fn window_ready(app: AppHandle) {
    if app.state::<WindowState>().shown.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Some(window) = app.get_webview_window(LABEL) {
        show(&window);
    }
}

pub fn on_event(window: &Window, event: &WindowEvent) {
    let WindowEvent::CloseRequested { api, .. } = event else { return };
    if window.label() != LABEL {
        return;
    }
    api.prevent_close();
    let _ = window.hide();

    let app = window.app_handle().clone();
    let opening = app.state::<WindowState>().openings.load(Ordering::SeqCst);
    thread::spawn(move || {
        thread::sleep(KEEP_ALIVE);
        if app.state::<WindowState>().openings.load(Ordering::SeqCst) != opening {
            return;
        }
        if let Some(window) = app.get_webview_window(LABEL) {
            let _ = window.destroy();
        }
    });
}

fn show(window: &WebviewWindow) {
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}
