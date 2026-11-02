//! The app's own settings, in a JSON file in its config folder. The display's settings live on the board instead, so
//! they follow it to another computer (docs/project.md, section 4).

use crate::display::texts::{ENGLISH, ITALIAN, Texts};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

const FILE_NAME: &str = "settings.json";

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Language {
    /// The system's, when there are words for it
    #[default]
    System,
    English,
    Italian,
}

impl Language {
    /// The language the app speaks: the one picked, or the system's.
    pub fn resolved(self) -> Self {
        match self {
            Self::System if sys_locale::get_locale().is_some_and(|locale| locale.starts_with("it")) => Self::Italian,
            Self::System => Self::English,
            picked => picked,
        }
    }

    pub fn texts(self) -> &'static Texts {
        match self.resolved() {
            Self::Italian => &ITALIAN,
            _ => &ENGLISH,
        }
    }
}

/// Where the overlay sits on the main screen
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Anchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    #[default]
    BottomRight,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct OverlaySettings {
    pub enabled: bool,
    pub anchor: Anchor,
    /// In percent
    pub opacity: u8,
    /// How long it stays after a page change or the knob let go
    pub seconds: u8,
    /// In percent of the full size
    pub size: u8,
}

impl OverlaySettings {
    /// Within the ranges the window offers
    pub fn checked(self) -> Self {
        Self {
            opacity: self.opacity.clamp(40, 100),
            seconds: self.seconds.clamp(1, 30),
            // Smaller, the names get too small to read on a screen at 100 %
            size: self.size.clamp(70, 100),
            ..self
        }
    }
}

impl Default for OverlaySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            anchor: Anchor::default(),
            opacity: 100,
            seconds: 4,
            size: 100,
        }
    }
}

/// Missing fields take their defaults, so a file from an older app still loads
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub language: Language,
    /// The LEDs' brightness, in percent: the board does not keep it
    pub led_brightness: u8,
    pub overlay: OverlaySettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: Language::default(),
            led_brightness: 50,
            overlay: OverlaySettings::default(),
        }
    }
}

impl Settings {
    /// Within the ranges the window offers
    fn checked(mut self) -> Self {
        self.led_brightness = self.led_brightness.clamp(10, 100);
        self.overlay = self.overlay.checked();
        self
    }
}

/// The settings as the window sees them
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    #[serde(flatten)]
    settings: Settings,
    /// The language the window speaks
    shown_language: Language,
}

impl From<Settings> for SettingsView {
    fn from(settings: Settings) -> Self {
        Self {
            shown_language: settings.language.resolved(),
            settings,
        }
    }
}

pub struct SettingsState(Mutex<Settings>);

impl SettingsState {
    pub fn get(&self) -> Settings {
        *self.0.lock().unwrap()
    }
}

/// The saved settings, or the defaults.
pub fn load(app: &AppHandle) -> SettingsState {
    let saved = path(app).and_then(|path| fs::read_to_string(path).ok());
    let settings = match saved.map(|text| serde_json::from_str::<Settings>(&text)) {
        Some(Ok(settings)) => settings.checked(),
        Some(Err(error)) => {
            eprintln!("unreadable settings, using the defaults: {error}");
            Settings::default()
        }
        None => Settings::default(),
    };
    SettingsState(Mutex::new(settings))
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    Some(app.path().app_config_dir().ok()?.join(FILE_NAME))
}

fn save(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let path = path(app).ok_or("no config folder")?;
    let folder = path.parent().ok_or("no config folder")?;
    fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    let text = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
    // Written aside first, so a crash never leaves half a file
    let written = path.with_extension("json.new");
    fs::write(&written, text).map_err(|error| error.to_string())?;
    fs::rename(&written, &path).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn settings(state: State<SettingsState>) -> SettingsView {
    state.get().into()
}

#[tauri::command]
pub fn set_settings(app: AppHandle, settings: Settings) -> Result<SettingsView, String> {
    let settings = settings.checked();
    *app.state::<SettingsState>().0.lock().unwrap() = settings;
    crate::device::apply_settings(&app, settings);
    crate::tray::set_language(&app, settings.language);
    let view = SettingsView::from(settings);
    let _ = app.emit("settings", view.clone());
    save(&app, &settings)?;
    Ok(view)
}

#[tauri::command]
pub fn about(app: AppHandle) -> About {
    About {
        version: app.package_info().version.to_string(),
        firmware: crate::firmware::VERSION.to_string(),
    }
}

#[derive(Serialize)]
pub struct About {
    version: String,
    /// The firmware the app gives the board
    firmware: String,
}

/// Starting with the system: None where the app cannot change it, as in a build that is not installed.
#[tauri::command]
pub fn start_at_login() -> Option<bool> {
    login::enabled()
}

#[tauri::command]
pub fn set_start_at_login(enabled: bool) -> Result<(), String> {
    login::set(enabled)
}

/// The login item the installer adds (windows/hooks.nsh), which Windows' startup apps can also turn off.
#[cfg(windows)]
mod login {
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        HKEY_CURRENT_USER, REG_BINARY, REG_SZ, RRF_RT_REG_BINARY, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW,
        RegSetKeyValueW,
    };

    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    /// Where Windows' startup apps keep the ones turned off: an odd first byte
    const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
    const NAME: &str = "Drezimo Soundboard";

    pub fn enabled() -> Option<bool> {
        installed_exe()?;
        if !value_exists(RUN, RRF_RT_REG_SZ) {
            return Some(false);
        }
        let mut state = [0u8; 64];
        let mut size = state.len() as u32;
        let read = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                wide(APPROVED).as_ptr(),
                wide(NAME).as_ptr(),
                RRF_RT_REG_BINARY,
                std::ptr::null_mut(),
                state.as_mut_ptr().cast(),
                &mut size,
            )
        };
        Some(read != ERROR_SUCCESS || state[0] % 2 == 0)
    }

    pub fn set(enabled: bool) -> Result<(), String> {
        let exe = installed_exe().ok_or("the app is not installed")?;
        if enabled {
            let command = format!("\"{}\" {}", exe.display(), crate::AUTOSTART_ARG);
            let bytes: Vec<u8> = wide(&command).iter().flat_map(|unit| unit.to_le_bytes()).collect();
            write(RUN, REG_SZ, &bytes)?;
            // On for the startup apps too
            delete(APPROVED);
            Ok(())
        } else {
            // Off as the startup apps turn it off: the installer writes the Run value again at each update
            write(APPROVED, REG_BINARY, &turned_off())
        }
    }

    /// What the startup apps keep for an app turned off: 3, then since when, as a FILETIME
    fn turned_off() -> [u8; 12] {
        let since_1601 =
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default() + Duration::from_secs(11_644_473_600);
        let ticks = (since_1601.as_nanos() / 100) as u64;
        let mut value = [0; 12];
        value[0] = 3;
        value[4..].copy_from_slice(&ticks.to_le_bytes());
        value
    }

    fn write(key: &str, kind: u32, data: &[u8]) -> Result<(), String> {
        let written = unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                wide(key).as_ptr(),
                wide(NAME).as_ptr(),
                kind,
                data.as_ptr().cast(),
                data.len() as u32,
            )
        };
        if written == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!("cannot write the login item (error {written})"))
        }
    }

    /// The app's path, when it runs from where the installer put it
    fn installed_exe() -> Option<PathBuf> {
        let exe = std::env::current_exe().ok()?;
        exe.with_file_name("uninstall.exe").is_file().then_some(exe)
    }

    fn value_exists(key: &str, kind: u32) -> bool {
        let found = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                wide(key).as_ptr(),
                wide(NAME).as_ptr(),
                kind,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        found == ERROR_SUCCESS
    }

    fn delete(key: &str) {
        unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, wide(key).as_ptr(), wide(NAME).as_ptr()) };
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain([0]).collect()
    }
}

/// No login item yet on macOS (docs/project.md, phase 5)
#[cfg(not(windows))]
mod login {
    pub fn enabled() -> Option<bool> {
        None
    }

    pub fn set(_enabled: bool) -> Result<(), String> {
        Err("not available on this system".to_string())
    }
}
