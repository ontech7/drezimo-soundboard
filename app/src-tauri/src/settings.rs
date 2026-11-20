//! The app's own settings, in a JSON file in its config folder. The display's settings live on the board instead, so
//! they follow it to another computer (docs/project.md, section 4).

use crate::display::texts::{ENGLISH, ITALIAN, Texts};
use crate::knob::KnobSettings;
use crate::press_again::{ByMode, PressAgain};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

pub const FILE_NAME: &str = "settings.json";

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

/// Longer than any screen's name the system gives
const MAX_SCREEN_NAME: usize = 100;

/// The screen the overlay shows on. A named one that is not connected gives way to the main screen.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OverlayScreen {
    /// The screen the mouse is on when the overlay appears
    #[default]
    Mouse,
    Main,
    /// As the system names it
    Named(String),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
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
    pub screen: OverlayScreen,
}

impl OverlaySettings {
    /// Within the ranges the window offers
    pub fn checked(self) -> Self {
        Self {
            opacity: self.opacity.clamp(40, 100),
            seconds: self.seconds.clamp(1, 30),
            // Smaller, the names get too small to read on a screen at 100 %
            size: self.size.clamp(70, 100),
            screen: match self.screen {
                OverlayScreen::Named(name) if name.chars().count() > MAX_SCREEN_NAME => OverlayScreen::Mouse,
                screen => screen,
            },
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
            screen: OverlayScreen::default(),
        }
    }
}

/// Missing fields take their defaults, so a file from an older app still loads
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub language: Language,
    /// The LEDs' brightness, in percent: the board does not keep it
    pub led_brightness: u8,
    /// What a pad does when pressed again, by its sound's mode, unless the user picked it for that pad
    #[serde(deserialize_with = "by_mode_or_default")]
    pub press_again_by_mode: ByMode,
    pub overlay: OverlaySettings,
    /// Shows the window's page of technical details
    pub diagnostics: bool,
    /// The welcome tour was seen through or skipped: until then the app opens its window at start, the tour on top
    pub tour_seen: bool,
    /// The notice that only the others hear the sounds, while Voicemod's Mute for me is on, was answered
    pub mute_for_me_notice_seen: bool,
    pub knob: KnobSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: Language::default(),
            led_brightness: 50,
            press_again_by_mode: ByMode::default(),
            overlay: OverlaySettings::default(),
            diagnostics: false,
            tour_seen: false,
            mute_for_me_notice_seen: false,
            knob: KnobSettings::default(),
        }
    }
}

impl Settings {
    /// Within the ranges the window offers
    pub fn checked(mut self) -> Self {
        self.led_brightness = self.led_brightness.clamp(10, 100);
        self.overlay = self.overlay.checked();
        self.knob = self.knob.checked();
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
        self.0.lock().unwrap().clone()
    }
}

/// The saved settings, or the defaults.
pub fn load(app: &AppHandle) -> SettingsState {
    let saved = path(app).and_then(|path| fs::read_to_string(path).ok());
    let settings = match saved.map(|text| read(&text)) {
        Some(Ok(settings)) => settings.checked(),
        Some(Err(error)) => {
            crate::errors::log(format!("unreadable settings, using the defaults: {error}"));
            Settings::default()
        }
        None => Settings::default(),
    };
    SettingsState(Mutex::new(settings))
}

/// A settings file, the single choice for every pad of the apps before the choice by mode included.
pub fn read(text: &str) -> Result<Settings, serde_json::Error> {
    let mut value: Value = serde_json::from_str(text)?;
    if let Some(fields) = value.as_object_mut()
        && !fields.contains_key("pressAgainByMode")
        && let Some(one) = fields.get("pressAgain").cloned()
        && let Ok(one) = PressAgain::deserialize(one)
    {
        let by_mode = serde_json::to_value(ByMode::from_one(one))?;
        fields.insert("pressAgainByMode".into(), by_mode);
    }
    let mut settings = Settings::deserialize(value)?;
    settings.press_again_by_mode = settings.press_again_by_mode.normalized();
    Ok(settings)
}

/// A broken choice by mode reads as the default, rather than losing every other setting.
fn by_mode_or_default<'de, D: Deserializer<'de>>(deserializer: D) -> Result<ByMode, D::Error> {
    Ok(ByMode::deserialize(Value::deserialize(deserializer)?).unwrap_or_default())
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    Some(app.path().app_config_dir().ok()?.join(FILE_NAME))
}

fn save(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let path = path(app).ok_or("no config folder")?;
    let folder = path.parent().ok_or("no config folder")?;
    fs::create_dir_all(folder).map_err(|error| crate::errors::in_english(&error))?;
    let text = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
    // Written aside first, so a crash never leaves half a file
    let written = path.with_extension("json.new");
    fs::write(&written, text).map_err(|error| crate::errors::in_english(&error))?;
    fs::rename(&written, &path).map_err(|error| crate::errors::in_english(&error))
}

#[tauri::command]
pub fn settings(state: State<SettingsState>) -> SettingsView {
    state.get().into()
}

#[tauri::command]
pub fn set_settings(app: AppHandle, settings: Settings) -> Result<SettingsView, String> {
    apply(&app, settings)
}

/// The settings in use at once, everywhere, then saved
pub fn apply(app: &AppHandle, settings: Settings) -> Result<SettingsView, String> {
    let settings = settings.checked();
    *app.state::<SettingsState>().0.lock().unwrap() = settings.clone();
    crate::device::apply_settings(app, settings.clone());
    crate::tray::set_language(app, settings.language);
    let view = SettingsView::from(settings.clone());
    let _ = app.emit("settings", view.clone());
    save(app, &settings).inspect_err(|error| crate::errors::log(format!("cannot save the settings: {error}")))?;
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
    use crate::wide;
    use std::path::PathBuf;
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_BINARY, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW,
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
            write_run(&format!("\"{}\" {}", exe.display(), crate::AUTOSTART_ARG))?;
        } else {
            delete(RUN);
        }
        // Turned off in the startup apps, the Run value alone would stay off
        delete(APPROVED);
        Ok(())
    }

    fn write_run(command: &str) -> Result<(), String> {
        let data: Vec<u8> = wide(command).iter().flat_map(|unit| unit.to_le_bytes()).collect();
        let written = unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                wide(RUN).as_ptr(),
                wide(NAME).as_ptr(),
                REG_SZ,
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
}

/// A launch agent: launchd reads its file at each login, so the file alone turns it on. System Settings can also turn it
/// off among the login items, which the app cannot see.
#[cfg(target_os = "macos")]
mod login {
    use super::{agent_program, launch_agent, stays_in_place};
    use std::path::PathBuf;
    use std::{fs, io};

    const FILE: &str = "dev.drezimo.soundboard.plist";

    pub fn enabled() -> Option<bool> {
        let exe = installed_exe()?;
        let agent = fs::read_to_string(agents_folder()?.join(FILE));
        // With another path in it, the app has moved since and the agent starts nothing
        Some(agent.is_ok_and(|agent| agent.contains(&agent_program(&exe))))
    }

    pub fn set(enabled: bool) -> Result<(), String> {
        let exe = installed_exe().ok_or("the app is not installed")?;
        let folder = agents_folder().ok_or("no home folder")?;
        let done = if enabled {
            fs::create_dir_all(&folder).and_then(|()| fs::write(folder.join(FILE), launch_agent(&exe)))
        } else {
            fs::remove_file(folder.join(FILE)).or_else(|error| match error.kind() {
                io::ErrorKind::NotFound => Ok(()),
                _ => Err(error),
            })
        };
        done.map_err(|error| format!("cannot change the login item: {error}"))
    }

    fn installed_exe() -> Option<String> {
        let exe = std::env::current_exe().ok()?;
        exe.to_str().filter(|path| stays_in_place(path)).map(String::from)
    }

    fn agents_folder() -> Option<PathBuf> {
        Some(PathBuf::from(std::env::var_os("HOME")?).join("Library/LaunchAgents"))
    }
}

/// What launchd runs at login: the app in the menu bar, with no window. Without KeepAlive, Quit stays quit.
#[cfg(any(target_os = "macos", test))]
fn launch_agent(exe: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Label</key>
	<string>dev.drezimo.soundboard</string>
	<key>ProgramArguments</key>
	<array>
		{program}
		<string>{autostart}</string>
	</array>
	<key>RunAtLoad</key>
	<true/>
	<key>ProcessType</key>
	<string>Interactive</string>
</dict>
</plist>
"#,
        program = agent_program(exe),
        autostart = crate::AUTOSTART_ARG
    )
}

/// The app's path, as the launch agent holds it
#[cfg(any(target_os = "macos", test))]
fn agent_program(exe: &str) -> String {
    let exe = exe.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    format!("<string>{exe}</string>")
}

/// An app bundle at a lasting path: not one still on its disk image, nor one that macOS runs from a random copy because
/// it was opened where it was downloaded.
#[cfg(any(target_os = "macos", test))]
fn stays_in_place(exe: &str) -> bool {
    exe.contains(".app/Contents/MacOS/") && !exe.starts_with("/Volumes/") && !exe.contains("/AppTranslocation/")
}

#[cfg(not(any(windows, target_os = "macos")))]
mod login {
    pub fn enabled() -> Option<bool> {
        None
    }

    pub fn set(_enabled: bool) -> Result<(), String> {
        Err("not available on this system".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_launch_agent_starts_the_app_with_no_window() {
        let agent = launch_agent("/Applications/R&B <1>.app/Contents/MacOS/drezimo");
        assert!(agent.contains("<string>/Applications/R&amp;B &lt;1&gt;.app/Contents/MacOS/drezimo</string>"));
        assert!(agent.contains("<string>--autostart</string>"));
        assert!(agent.contains("<key>RunAtLoad</key>\n\t<true/>"));
        assert!(!agent.contains("KeepAlive"));
    }

    #[test]
    fn only_a_bundle_at_a_lasting_path_can_start_at_login() {
        let in_bundle = "Drezimo Soundboard.app/Contents/MacOS/drezimo";
        assert!(stays_in_place(&format!("/Applications/{in_bundle}")));
        assert!(stays_in_place(&format!("/Users/me/Applications/{in_bundle}")));
        assert!(!stays_in_place("/Users/me/drezimo/app/src-tauri/target/debug/drezimo"));
        assert!(!stays_in_place(&format!("/Volumes/Drezimo Soundboard/{in_bundle}")));
        assert!(!stays_in_place(&format!(
            "/private/var/folders/ab/T/AppTranslocation/1234/d/{in_bundle}"
        )));
    }

    #[test]
    fn a_screen_name_longer_than_any_screens_is_the_mouses() {
        let named = |length: usize| OverlaySettings {
            screen: OverlayScreen::Named("s".repeat(length)),
            ..OverlaySettings::default()
        };
        assert_eq!(named(100).checked().screen, named(100).screen);
        assert_eq!(named(101).checked().screen, OverlayScreen::Mouse);
    }

    #[test]
    fn an_older_file_loads_with_the_new_fields() {
        let older = r#"{
            "language": "italian",
            "ledBrightness": 70,
            "overlay": { "enabled": true, "anchor": "top", "opacity": 90, "seconds": 5, "size": 80 },
            "diagnostics": true,
            "tourSeen": true,
            "pressAgain": "bounce"
        }"#;
        let settings: Settings = serde_json::from_str(older).unwrap();
        assert_eq!(settings.language, Language::Italian);
        assert_eq!(settings.led_brightness, 70);
        assert_eq!(settings.overlay.anchor, Anchor::Top);
        assert!(settings.tour_seen);
        assert!(!settings.mute_for_me_notice_seen);
        assert_eq!(settings.press_again_by_mode, ByMode::default());
        assert_eq!(settings.overlay.screen, OverlayScreen::Mouse);
        assert_eq!(settings.knob, KnobSettings::default());
    }

    #[test]
    fn the_overlays_screen_keeps_its_json_forms() {
        for (screen, json) in [
            (OverlayScreen::Mouse, r#""mouse""#),
            (OverlayScreen::Main, r#""main""#),
            (OverlayScreen::Named("DELL U2720Q".into()), r#"{"named":"DELL U2720Q"}"#),
        ] {
            assert_eq!(serde_json::to_string(&screen).unwrap(), json);
            assert_eq!(serde_json::from_str::<OverlayScreen>(json).unwrap(), screen);
        }
    }

    #[test]
    fn an_old_single_choice_migrates_and_is_not_written_again() {
        let settings = read(r#"{"pressAgain": "stop"}"#).unwrap();
        assert_eq!(settings.press_again_by_mode, ByMode::from_one(PressAgain::Stop));
        let written = serde_json::to_value(&settings).unwrap();
        assert!(written.get("pressAgain").is_none());
        assert!(written.get("pressAgainByMode").is_some());
        // Both present: the new one wins
        let both = read(r#"{"pressAgain": "stop", "pressAgainByMode": {}}"#).unwrap();
        assert_eq!(both.press_again_by_mode, ByMode::default());
    }

    #[test]
    fn a_choice_by_mode_reads_as_the_window_offers_it() {
        let settings = read(r#"{"pressAgainByMode": {"playRestart": "restart", "playPause": "stop"}}"#).unwrap();
        assert_eq!(settings.press_again_by_mode.play_restart, PressAgain::Voicemod);
        assert_eq!(settings.press_again_by_mode.play_pause, PressAgain::Stop);
    }

    #[test]
    fn a_broken_choice_by_mode_keeps_the_other_settings() {
        for broken in ["null", r#""stop""#, "[1, 2]"] {
            let settings = read(&format!(r#"{{"ledBrightness": 30, "pressAgainByMode": {broken}}}"#)).unwrap();
            assert_eq!(settings.led_brightness, 30, "{broken}");
            assert_eq!(settings.press_again_by_mode, ByMode::default(), "{broken}");
        }
    }
}
