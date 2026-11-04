//! The report the Diagnostics page copies, for the owner to paste in a chat: versions, connections, settings and the
//! latest errors, in English, without paths or names.

use crate::board::BoardStatus;
use crate::custom_pages::CustomPagesState;
use crate::device::Status;
use crate::errors::Error;
use crate::firmware::FirmwareUpdate;
use crate::macros::MacrosState;
use crate::press_again::{ByMode, PressAgain};
use crate::settings::{OverlayScreen, Settings, SettingsState};
use crate::updates::{UpdateState, UpdateStateHolder};
use crate::voicemod::{License, VoicemodStatus};
use chrono::{DateTime, Local, TimeZone};
use std::fmt::Write;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

pub struct Facts {
    pub now: DateTime<Local>,
    pub app_version: String,
    /// Built by the release workflow
    pub official: bool,
    pub os: String,
    pub settings: Settings,
    pub status: Status,
    pub update: UpdateState,
    /// None where the app cannot change it: a build not installed, or macOS
    pub start_at_login: Option<bool>,
    pub own_pages: usize,
    pub macros: usize,
    pub errors: Vec<Error>,
}

#[tauri::command]
pub fn report(app: AppHandle) -> String {
    // Not held while the rest is gathered and written: the device thread publishes it
    let status = app.state::<Mutex<Status>>().lock().unwrap().clone();
    write(&Facts {
        now: Local::now(),
        app_version: app.package_info().version.to_string(),
        official: crate::updates::OFFICIAL,
        os: os_description(),
        settings: app.state::<SettingsState>().get(),
        status,
        update: app.state::<UpdateStateHolder>().get(),
        start_at_login: crate::settings::start_at_login(),
        own_pages: app.state::<CustomPagesState>().get().len(),
        macros: app.state::<MacrosState>().get().len(),
        errors: crate::errors::recent(),
    })
}

fn write(facts: &Facts) -> String {
    let Facts { status, settings, .. } = facts;
    let mut text = String::new();
    let mut line = |line: String| {
        text.push_str(&line);
        text.push('\n');
    };
    line(format!(
        "Drezimo Soundboard report, {}",
        facts.now.format("%Y-%m-%d %H:%M")
    ));
    let build = if facts.official {
        "published build"
    } else {
        "local build"
    };
    line(format!("App: {} ({build})", facts.app_version));
    line(format!("System: {}", facts.os));
    line(format!(
        "Language: {:?} (shown in {:?})",
        settings.language,
        settings.language.resolved()
    ));
    line(format!("Board: {}", board(&status.board)));
    if let Some(update) = &status.firmware_update {
        line(format!("Firmware update: {}", firmware_update(update)));
    }
    line(format!("Voicemod: {}", voicemod(status)));
    let pages = status
        .page
        .as_ref()
        .map_or("none yet".to_string(), |page| page.count.to_string());
    let macros = if facts.macros == 1 { "macro" } else { "macros" };
    line(format!(
        "Pages: {pages} ({} of your own), {} {macros}",
        facts.own_pages, facts.macros
    ));
    line(format!("Updates: {}", update_text(&facts.update)));
    let start_at_login = match facts.start_at_login {
        Some(true) => "on",
        Some(false) => "off",
        None => "not available",
    };
    line(format!("Start at login: {start_at_login}"));
    line(format!("Settings: {}", settings_text(settings, status)));
    if facts.errors.is_empty() {
        line("Recent errors: none".to_string());
    } else {
        line("Recent errors:".to_string());
        for error in &facts.errors {
            let mut text = format!("  {} {}", error.first.format("%H:%M:%S"), without_paths(&error.message));
            if error.count > 1 {
                let _ = write!(text, " (×{}, last {})", error.count, error.last.format("%H:%M:%S"));
            }
            line(text);
        }
    }
    text
}

fn update_text(update: &UpdateState) -> String {
    let at = |millis: i64| {
        Local
            .timestamp_millis_opt(millis)
            .single()
            .map_or_else(String::new, |at| at.format("%Y-%m-%d %H:%M").to_string())
    };
    match update {
        UpdateState::Local => "development build, none".to_string(),
        UpdateState::Unavailable => "the updater could not start".to_string(),
        UpdateState::Waiting => "waiting for a moment nobody uses the app".to_string(),
        UpdateState::Checking => "looking".to_string(),
        UpdateState::UpToDate { checked_at } => format!("up to date, checked {}", at(*checked_at)),
        UpdateState::Downloading { version } => format!("downloading {version}"),
        UpdateState::Ready { version } => format!("{version} ready to install"),
        UpdateState::Failed { at: failed } => format!("failed at {}", at(*failed)),
    }
}

fn screen_text(screen: &OverlayScreen) -> String {
    match screen {
        OverlayScreen::Mouse => "the screen with the mouse".to_string(),
        OverlayScreen::Main => "the main screen".to_string(),
        OverlayScreen::Named(name) => format!("screen {name}"),
    }
}

fn board(board: &BoardStatus) -> String {
    match board {
        BoardStatus::Searching => "not connected".to_string(),
        BoardStatus::Busy { port } => format!("in use by another program on {port}"),
        BoardStatus::Connected { port, firmware } => format!("connected on {port}, firmware {firmware}"),
    }
}

fn firmware_update(update: &FirmwareUpdate) -> String {
    match update {
        FirmwareUpdate::Updating { from, to } => format!("updating from {from} to {to}"),
        FirmwareUpdate::Restarting { to } => format!("restarting with {to}"),
        FirmwareUpdate::Failed { reason } => format!("failed: {}", without_paths(reason)),
    }
}

fn voicemod(status: &Status) -> String {
    match status.voicemod {
        VoicemodStatus::Connecting => "connecting".to_string(),
        VoicemodStatus::Unreachable => "not running".to_string(),
        VoicemodStatus::KeyRejected => "refused the client key".to_string(),
        VoicemodStatus::MissingKey => "built without a client key".to_string(),
        VoicemodStatus::Connected => {
            let mut text = "connected".to_string();
            if let Some(version) = &status.voicemod_version {
                let _ = write!(text, ", {version}");
            }
            match status.license {
                Some(License::Free) => text.push_str(", free licence"),
                Some(License::Pro) => text.push_str(", Pro licence"),
                None => {}
            }
            text.push_str(if status.voicemod_log {
                ", log read"
            } else {
                ", log not read"
            });
            text.push_str(if status.single_sound_stop {
                ", single sound stop"
            } else {
                ", no single sound stop"
            });
            text
        }
    }
}

fn press_again_text(by_mode: &ByMode) -> String {
    let own: Vec<String> = ByMode::MODES
        .into_iter()
        .filter(|&mode| by_mode.get(mode) != PressAgain::Voicemod)
        .map(|mode| format!("{mode:?} {:?}", by_mode.get(mode)))
        .collect();
    if own.is_empty() {
        "Voicemod".to_string()
    } else {
        format!("{}, else Voicemod", own.join(", "))
    }
}

fn settings_text(settings: &Settings, status: &Status) -> String {
    let overlay = &settings.overlay;
    let mut text = format!(
        "LEDs {} %, pads pressed again: {}",
        settings.led_brightness,
        press_again_text(&settings.press_again_by_mode)
    );
    if overlay.enabled {
        let _ = write!(
            text,
            ", overlay {:?} on {} at {} % size, {} % opacity, {} s",
            overlay.anchor,
            screen_text(&overlay.screen),
            overlay.size,
            overlay.opacity,
            overlay.seconds
        );
    } else {
        text.push_str(", overlay off");
    }
    if let Some(display) = &status.display_settings {
        let _ = write!(
            text,
            ", display {} %, dims after {} min, off after {} min, shifts every {} min",
            display.brightness, display.dim_after, display.off_after, display.shift_every
        );
    }
    text
}

/// A user's folder names the user: only the file name stays.
fn without_paths(message: &str) -> String {
    let home = ["/Users/", "/home/", r"\Users\"]
        .iter()
        .filter_map(|marker| message.find(marker))
        .min();
    let Some(mut start) = home else {
        return message.to_string();
    };
    // A Windows path starts with its drive
    if let Some(drive) = message[..start]
        .strip_suffix(':')
        .and_then(|before| before.char_indices().last())
    {
        start = drive.0;
    }
    let end = message.rfind(['/', '\\']).map_or(message.len(), |last| last + 1);
    format!("{}{}", &message[..start], &message[end..])
}

/// The system's name and version, as a person would say it.
fn os_description() -> String {
    format!("{} ({})", os_version(), std::env::consts::ARCH)
}

#[cfg(target_os = "macos")]
fn os_version() -> String {
    let plist = std::fs::read_to_string("/System/Library/CoreServices/SystemVersion.plist").unwrap_or_default();
    let version = plist
        .split("<key>ProductVersion</key>")
        .nth(1)
        .and_then(|rest| rest.split("<string>").nth(1))
        .and_then(|rest| rest.split("</string>").next());
    match version {
        Some(version) => format!("macOS {version}"),
        None => "macOS".to_string(),
    }
}

#[cfg(windows)]
fn os_version() -> String {
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};

    let read = |name: &str| -> Option<String> {
        let key = crate::wide(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion");
        let name = crate::wide(name);
        let mut buffer = [0u16; 64];
        let mut size = (buffer.len() * 2) as u32;
        let found = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_SZ,
                std::ptr::null_mut(),
                buffer.as_mut_ptr().cast(),
                &mut size,
            )
        };
        (found == ERROR_SUCCESS).then(|| String::from_utf16_lossy(&buffer[..(size as usize / 2).saturating_sub(1)]))
    };
    let build = read("CurrentBuild").unwrap_or_default();
    // Windows 11 still calls itself Windows 10 there
    let name = if build.parse::<u32>().is_ok_and(|build| build >= 22000) {
        "Windows 11"
    } else {
        "Windows 10"
    };
    match read("DisplayVersion") {
        Some(release) => format!("{name} {release}, build {build}"),
        None => format!("{name}, build {build}"),
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
fn os_version() -> String {
    std::env::consts::OS.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::BoardStatus;
    use crate::controller::PageSummary;
    use crate::press_again::PressAgain;
    use crate::settings::OverlayScreen;
    use crate::voicemod::{License, VoicemodStatus};
    use chrono::TimeZone;

    fn facts() -> Facts {
        let at = |h, m, s| Local.with_ymd_and_hms(2026, 11, 3, h, m, s).unwrap();
        let status = Status {
            board: BoardStatus::Connected {
                port: "COM3".into(),
                firmware: "0.2.1".into(),
            },
            voicemod: VoicemodStatus::Connected,
            voicemod_log: true,
            license: Some(License::Free),
            voicemod_version: Some("3.17.43".into()),
            page: Some(PageSummary {
                name: "Pirates".into(),
                part: 1,
                parts: 1,
                number: 10,
                count: 44,
            }),
            ..Status::default()
        };
        Facts {
            now: at(13, 5, 0),
            app_version: "0.1.0".into(),
            official: false,
            os: "macOS 26.0 (aarch64)".into(),
            settings: Settings::default(),
            status,
            update: UpdateState::UpToDate {
                checked_at: at(12, 40, 0).timestamp_millis(),
            },
            start_at_login: Some(false),
            own_pages: 2,
            macros: 1,
            errors: vec![
                error(at(12, 50, 21), 3, "lost the connection to Voicemod: closed"),
                error(
                    at(12, 51, 2),
                    1,
                    "unreadable pages: /Users/someone/Library/Application Support/x/pages.json",
                ),
                error(
                    at(12, 52, 3),
                    1,
                    r"cannot open C:\Users\someone\AppData\Local\x\settings.json",
                ),
            ],
        }
    }

    /// Repeated every 2.5 minutes
    fn error(first: DateTime<Local>, count: u32, message: &str) -> Error {
        Error {
            message: message.into(),
            first,
            last: first + chrono::Duration::seconds(150 * i64::from(count - 1)),
            count,
        }
    }

    #[test]
    fn writes_the_facts_without_paths() {
        let report = write(&facts());
        for line in [
            "Drezimo Soundboard report, 2026-11-03 13:05",
            "App: 0.1.0 (local build)",
            "System: macOS 26.0 (aarch64)",
            "Board: connected on COM3, firmware 0.2.1",
            "Voicemod: connected, 3.17.43, free licence, log read, no single sound stop",
            "Pages: 44 (2 of your own), 1 macro",
            "Updates: up to date, checked 2026-11-03 12:40",
            "Start at login: off",
            "LEDs 50 %, pads pressed again: Voicemod",
            "overlay BottomRight on the screen with the mouse",
            "12:50:21 lost the connection to Voicemod: closed (×3, last 12:55:21)",
            "12:51:02 unreadable pages: pages.json\n",
            "12:52:03 cannot open settings.json",
        ] {
            assert!(report.contains(line), "missing {line:?} in:\n{report}");
        }
        assert!(!report.contains("someone"), "a path in:\n{report}");
    }

    #[test]
    fn names_the_overlays_screen() {
        let mut facts = facts();
        facts.settings.overlay.screen = OverlayScreen::Named(r"\\.\DISPLAY2".into());
        let report = write(&facts);
        assert!(
            report.contains(r"overlay BottomRight on screen \\.\DISPLAY2"),
            "{report}"
        );
    }

    #[test]
    fn names_only_the_modes_with_their_own_choice() {
        let mut facts = facts();
        facts.settings.press_again_by_mode.play_pause = PressAgain::Stop;
        facts.settings.press_again_by_mode.play_loop_on_press = PressAgain::Restart;
        let report = write(&facts);
        assert!(
            report.contains("pads pressed again: PlayPause Stop, PlayLoopOnPress Restart, else Voicemod"),
            "{report}"
        );
    }

    #[test]
    fn cuts_a_path_whatever_comes_before_it() {
        assert_eq!(without_paths(r"è:\Users\someone\x.txt"), "x.txt");
        assert_eq!(without_paths("€/Users/someone/x.txt"), "€x.txt");
    }

    #[test]
    fn says_when_there_is_no_error() {
        let report = write(&Facts {
            errors: Vec::new(),
            ..facts()
        });
        assert!(report.contains("Recent errors: none"));
    }
}
