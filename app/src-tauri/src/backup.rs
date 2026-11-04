//! The configuration as a file of its own, to keep aside or take to another PC: exported plainly, imported only after
//! the same checks as the app's own files (docs/project.md, section 8).

use crate::custom_pages::{self, Content, CustomPage, CustomPagesState};
use crate::knob::{KnobSettings, PressAction};
use crate::macros::{self, Macro, MacrosState, Step};
use crate::press_again::ByMode;
use crate::protocol::DisplaySettings;
use crate::settings::{self, Language, OverlaySettings, Settings, SettingsState};
use crate::{device, errors};
use chrono::{DateTime, Local, SecondsFormat};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::fmt::Display;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::{fs, io};
use tauri::{AppHandle, Manager, State};

pub const FORMAT: u32 = 1;
pub const MAX_BYTES: usize = 5 * 1024 * 1024;
/// What the file says it is
const MARKER: &str = "configuration";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Exported<'a> {
    drezimo: &'static str,
    format: u32,
    app_version: &'a str,
    exported_at: String,
    settings: SharedSettings,
    pages: &'a [CustomPage],
    macros: &'a [Macro],
    #[serde(skip_serializing_if = "Option::is_none")]
    display: Option<DisplaySettings>,
}

/// The settings that travel; the others belong to one PC: the tour seen, the notices answered, Diagnostics shown
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedSettings {
    pub language: Language,
    pub led_brightness: u8,
    pub press_again_by_mode: ByMode,
    pub knob: KnobSettings,
    pub overlay: OverlaySettings,
}

impl SharedSettings {
    pub fn of(settings: &Settings) -> Self {
        Self {
            language: settings.language,
            led_brightness: settings.led_brightness,
            press_again_by_mode: settings.press_again_by_mode,
            knob: settings.knob.clone(),
            overlay: settings.overlay.clone(),
        }
    }

    /// These settings in place of `settings`' own, its PC's own kept
    pub fn onto(self, settings: Settings) -> Settings {
        Settings {
            language: self.language,
            led_brightness: self.led_brightness,
            press_again_by_mode: self.press_again_by_mode,
            knob: self.knob,
            overlay: self.overlay,
            ..settings
        }
        .checked()
    }

    /// Field by field, so that an unreadable one takes its default alone; numbers come back within the window's ranges
    fn read(value: &Value) -> Self {
        let defaults = Settings::default();
        let brightness = value["ledBrightness"].as_f64();
        Self {
            language: field(value, "language").unwrap_or_default(),
            led_brightness: brightness.map_or(defaults.led_brightness, |percent| {
                percent.clamp(10.0, 100.0).round() as u8
            }),
            press_again_by_mode: field::<ByMode>(value, "pressAgainByMode")
                .unwrap_or_default()
                .normalized(),
            knob: field(value, "knob").unwrap_or_default(),
            overlay: field::<OverlaySettings>(value, "overlay").unwrap_or_default().checked(),
        }
    }
}

fn field<T: DeserializeOwned>(value: &Value, name: &str) -> Option<T> {
    T::deserialize(value.get(name)?).ok()
}

pub fn export(
    settings: &Settings,
    pages: &[CustomPage],
    macros: &[Macro],
    display: Option<DisplaySettings>,
    app_version: &str,
    now: DateTime<Local>,
) -> String {
    let exported = Exported {
        drezimo: MARKER,
        format: FORMAT,
        app_version,
        exported_at: now.to_rfc3339_opts(SecondsFormat::Secs, false),
        settings: SharedSettings::of(settings),
        pages,
        macros,
        display,
    };
    serde_json::to_string_pretty(&exported).unwrap_or_default()
}

#[derive(Debug, PartialEq)]
pub enum Refused {
    TooBig,
    NotJson,
    NotDrezimo,
    /// From a newer app, with a format this one does not know
    Newer,
}

/// What the checks left out, counted
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dropped {
    pub pages: usize,
    pub pads: usize,
    pub macros: usize,
    pub steps: usize,
    /// Parts unusable as a whole
    pub parts: usize,
}

/// A file that passed the checks: each part it holds, as the app's own files would hold it
#[derive(Debug, PartialEq)]
pub struct Checked {
    pub app_version: String,
    pub exported_at: String,
    pub settings: Option<SharedSettings>,
    pub pages: Option<Vec<CustomPage>>,
    pub macros: Option<Vec<Macro>>,
    pub display: Option<DisplaySettings>,
    pub dropped: Dropped,
}

/// Reads a file, refusing it whole only when it is no configuration of this app or of an older one: a part that cannot
/// be used is left out alone, and what the app's own rules leave out is counted.
pub fn check(bytes: &[u8]) -> Result<Checked, Refused> {
    if bytes.len() > MAX_BYTES {
        return Err(Refused::TooBig);
    }
    let mut file: Value = serde_json::from_slice(bytes).map_err(|_| Refused::NotJson)?;
    if file["drezimo"] != MARKER {
        return Err(Refused::NotDrezimo);
    }
    let format = file["format"].as_u64().ok_or(Refused::NotDrezimo)?;
    if format > u64::from(FORMAT) {
        return Err(Refused::Newer);
    }

    let mut dropped = Dropped::default();
    // A part of the wrong kind cannot be used; one left out is simply not there
    let mut part = |name: &str, list: bool| {
        let value = file[name].take();
        let usable = if list { value.is_array() } else { value.is_object() };
        if !usable && !value.is_null() {
            dropped.parts += 1;
        }
        usable.then_some(value)
    };
    let settings = part("settings", false);
    let pages = part("pages", true);
    let macros = part("macros", true);
    let display = part("display", false);

    let pages = pages.map(|values| {
        let (pages, pages_dropped, pads_dropped) = custom_pages::from_values(list(values));
        dropped.pages = pages_dropped;
        dropped.pads = pads_dropped;
        pages
    });
    let macros = macros.map(|values| {
        let (macros, macros_dropped, steps_dropped) = macros::from_values(list(values));
        dropped.macros = macros_dropped;
        dropped.steps = steps_dropped;
        macros
    });
    Ok(Checked {
        app_version: text(&file["appVersion"], 32),
        exported_at: text(&file["exportedAt"], 40),
        settings: settings.as_ref().map(SharedSettings::read),
        pages,
        macros,
        display: display.as_ref().map(display_settings),
        dropped,
    })
}

/// The parts the user chose to import
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Parts {
    pub settings: bool,
    pub pages: bool,
    pub macros: bool,
    pub display: bool,
}

pub struct Merged {
    pub settings: Settings,
    pub pages: Vec<CustomPage>,
    pub macros: Vec<Macro>,
    pub display: Option<DisplaySettings>,
    /// References to the user's pages or macros that will not exist, cut: knob gestures, macro pads, go-to-page pads
    /// and steps
    pub cut: usize,
}

fn list(value: Value) -> Vec<Value> {
    match value {
        Value::Array(items) => items,
        _ => Vec::new(),
    }
}

fn text(value: &Value, max: usize) -> String {
    value.as_str().unwrap_or_default().chars().take(max).collect()
}

/// The values the window offers (app/src/pages/BoardPage.tsx), in minutes
const DIM_AFTER: [u16; 6] = [0, 1, 2, 5, 10, 30];
const OFF_AFTER: [u16; 6] = [0, 5, 10, 15, 30, 60];
const SHIFT_EVERY: [u16; 5] = [0, 1, 3, 5, 10];

/// Each value the nearest the window offers, the window's own default when missing
fn display_settings(value: &Value) -> DisplaySettings {
    let number = |name: &str| value[name].as_f64();
    DisplaySettings {
        brightness: number("brightness").map_or(50, |percent| percent.clamp(0.0, 100.0).round() as u8),
        dim_after: nearest(number("dimAfter"), &DIM_AFTER, 1),
        off_after: nearest(number("offAfter"), &OFF_AFTER, 10),
        shift_every: nearest(number("shiftEvery"), &SHIFT_EVERY, 3),
    }
}

fn nearest(value: Option<f64>, allowed: &[u16], default: u16) -> u16 {
    let Some(value) = value else { return default };
    let distance = |minutes: &u16| (f64::from(*minutes) - value).abs();
    allowed
        .iter()
        .copied()
        .min_by(|a, b| distance(a).total_cmp(&distance(b)))
        .unwrap_or(default)
}

/// The configuration as it will be, from the parts chosen and the ones kept.
pub fn merge(checked: &Checked, parts: Parts, current: (&Settings, &[CustomPage], &[Macro])) -> Merged {
    let (settings, pages, macros) = current;
    // The parts that come from the file
    let imported = Parts {
        settings: parts.settings && checked.settings.is_some(),
        pages: parts.pages && checked.pages.is_some(),
        macros: parts.macros && checked.macros.is_some(),
        display: parts.display && checked.display.is_some(),
    };
    let mut new_settings = match &checked.settings {
        Some(shared) if imported.settings => shared.clone().onto(settings.clone()),
        _ => settings.clone(),
    };
    let mut new_pages = match &checked.pages {
        Some(file) if imported.pages => file.clone(),
        _ => pages.to_vec(),
    };
    let mut new_macros = match &checked.macros {
        Some(file) if imported.macros => file.clone(),
        _ => macros.to_vec(),
    };
    let before = Ids::of(pages, macros);
    let cut = cut_missing(
        &mut new_settings.knob,
        &mut new_pages,
        &mut new_macros,
        &before,
        imported,
    );
    Merged {
        settings: new_settings,
        pages: new_pages,
        macros: new_macros,
        display: checked.display.filter(|_| imported.display),
        cut,
    }
}

/// The user's pages and macros, by id
struct Ids {
    pages: HashSet<String>,
    macros: HashSet<String>,
}

impl Ids {
    fn of(pages: &[CustomPage], macros: &[Macro]) -> Self {
        Self {
            pages: pages.iter().map(|page| page.id.clone()).collect(),
            macros: macros.iter().map(|item| item.id.clone()).collect(),
        }
    }
}

/// Cuts what the import breaks: a reference to one of the user's pages or macros that will not be there, when it came
/// with the file or what it points to was there before. One broken already stays, shown missing in the window as
/// before; a soundboard of Voicemod's is on the PC or not, as for any page, so it stays.
fn cut_missing(
    knob: &mut KnobSettings,
    pages: &mut [CustomPage],
    macros: &mut [Macro],
    before: &Ids,
    imported: Parts,
) -> usize {
    let after = Ids::of(pages, macros);
    let breaks = |id: &str, after: &HashSet<String>, before: &HashSet<String>, from_file: bool| {
        !id.is_empty() && !after.contains(id) && (from_file || before.contains(id))
    };
    let page_breaks = |id: &str, from_file| {
        id.starts_with(custom_pages::ID_PREFIX) && breaks(id, &after.pages, &before.pages, from_file)
    };
    let macro_breaks = |id: &str, from_file| breaks(id, &after.macros, &before.macros, from_file);
    let mut cut = 0;
    for gesture in [&mut knob.tap, &mut knob.double_tap, &mut knob.long_press] {
        let broken = match gesture {
            PressAction::Page { page } => page_breaks(page, imported.settings),
            PressAction::Macro { macro_id } => macro_breaks(macro_id, imported.settings),
            _ => false,
        };
        if broken {
            *gesture = PressAction::Nothing;
            cut += 1;
        }
    }
    for pad in pages.iter_mut().flat_map(|page| page.pads.iter_mut()) {
        let broken = match pad.as_ref().map(|pad| &pad.content) {
            Some(Content::Macro { id }) => macro_breaks(id, imported.pages),
            Some(Content::Action { page: Some(page), .. }) => page_breaks(page, imported.pages),
            _ => false,
        };
        if broken {
            *pad = None;
            cut += 1;
        }
    }
    for item in macros.iter_mut() {
        let steps = item.steps.len();
        item.steps
            .retain(|step| !matches!(step, Step::GoToPage { page } if page_breaks(page, imported.macros)));
        cut += steps - item.steps.len();
    }
    cut
}

/// What the app had before the last import
#[derive(Debug, PartialEq)]
pub struct Before {
    pub settings: Settings,
    pub pages: Vec<CustomPage>,
    pub macros: Vec<Macro>,
    pub display: Option<DisplaySettings>,
}

/// The copy of what the app had before an import, in the app's config folder
const BEFORE_IMPORT: &str = "before-import";
const DISPLAY_FILE: &str = "display.json";

/// The copy of what the app has now, written aside: it takes the older one's place only once the import is done, so a
/// failed import keeps the copy before it, and a copy is never half one and half the other.
pub fn write_copy(
    folder: &Path,
    settings: &Settings,
    pages: &[CustomPage],
    macros: &[Macro],
    display: Option<DisplaySettings>,
) -> io::Result<()> {
    let written = written_copy(folder);
    if written.exists() {
        fs::remove_dir_all(&written)?;
    }
    fs::create_dir_all(&written)?;
    fs::write(
        written.join(settings::FILE_NAME),
        serde_json::to_string_pretty(settings)?,
    )?;
    fs::write(
        written.join(custom_pages::FILE_NAME),
        serde_json::to_string_pretty(pages)?,
    )?;
    fs::write(written.join(macros::FILE_NAME), serde_json::to_string_pretty(macros)?)?;
    if let Some(display) = display {
        fs::write(written.join(DISPLAY_FILE), serde_json::to_string_pretty(&display)?)?;
    }
    Ok(())
}

fn written_copy(folder: &Path) -> PathBuf {
    folder.join(format!("{BEFORE_IMPORT}.new"))
}

/// The copy, read with the app's own rules; none if a file of it cannot be read, rather than putting back less
pub fn before_import(folder: &Path) -> Option<Before> {
    let copy = folder.join(BEFORE_IMPORT);
    let read = |name: &str| fs::read_to_string(copy.join(name)).ok();
    let settings = settings::read(&read(settings::FILE_NAME)?).ok()?.checked();
    let pages = serde_json::from_str(&read(custom_pages::FILE_NAME)?).ok()?;
    let macros = serde_json::from_str(&read(macros::FILE_NAME)?).ok()?;
    let display = match copy.join(DISPLAY_FILE).exists() {
        true => Some(serde_json::from_str::<Value>(&read(DISPLAY_FILE)?).ok()?),
        false => None,
    };
    Some(Before {
        settings,
        pages: custom_pages::from_values(pages).0,
        macros: macros::from_values(macros).0,
        display: display.as_ref().map(display_settings),
    })
}

/// The copy written aside in place of the older one
pub fn keep_copy(folder: &Path) -> io::Result<()> {
    let copy = folder.join(BEFORE_IMPORT);
    if copy.exists() {
        fs::remove_dir_all(&copy)?;
    }
    fs::rename(written_copy(folder), copy)
}

pub fn drop_copy(folder: &Path) {
    let _ = fs::remove_dir_all(written_copy(folder));
}

pub fn has_before_import_in(folder: &Path) -> bool {
    folder.join(BEFORE_IMPORT).join(settings::FILE_NAME).exists()
}

fn write_atomically(path: &Path, text: &str) -> io::Result<()> {
    let mut written = path.as_os_str().to_owned();
    written.push(".new");
    fs::write(&written, text)?;
    fs::rename(&written, path).inspect_err(|_| {
        let _ = fs::remove_file(&written);
    })
}

/// A file read and checked; the error is the key of the window's sentence
pub fn read_file(path: &Path) -> Result<Checked, String> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .and_then(|file| file.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes))
        .map_err(|_| "cannotRead".to_string())?;
    check(&bytes).map_err(|refused| {
        match refused {
            Refused::TooBig => "tooBig",
            Refused::NotJson => "notJson",
            Refused::NotDrezimo => "notDrezimo",
            Refused::Newer => "newer",
        }
        .to_string()
    })
}

/// A file checked, until the user imports it or not
#[derive(Default)]
pub struct ImportState(Mutex<Option<Checked>>);

impl ImportState {
    fn keep(&self, checked: Checked) {
        *self.0.lock().unwrap() = Some(checked);
    }

    fn take(&self) -> Result<Checked, String> {
        self.0
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| "nothingToImport".to_string())
    }
}

/// What the confirmation shows of a file that passed the checks
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    exported_at: String,
    app_version: String,
    settings: bool,
    pages: Option<usize>,
    macros: Option<usize>,
    display: bool,
    dropped: Dropped,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    cut: usize,
    /// Display settings to put on Drezimo while it is not connected
    display_skipped: bool,
}

fn config_folder(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_config_dir().map_err(|_| "cannotWrite".to_string())
}

/// The file's name only, for the error list: its folder may hold the user's name
fn logged(action: &str, path: &Path, error: impl Display) {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    errors::log(format!("cannot {action} the configuration {name}: {error}"));
}

// The file work runs off the main thread, so that a big file never holds up the window or the tray
#[tauri::command(async)]
pub fn export_configuration(app: AppHandle, path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    let text = export(
        &app.state::<SettingsState>().get(),
        &app.state::<CustomPagesState>().get(),
        &app.state::<MacrosState>().get(),
        device::display_settings(&app),
        &app.package_info().version.to_string(),
        Local::now(),
    );
    write_atomically(&path, &text).map_err(|error| {
        logged("export", &path, errors::in_english(&error));
        "cannotWrite".to_string()
    })
}

#[tauri::command(async)]
pub fn check_import(app: AppHandle, path: String) -> Result<ImportPreview, String> {
    let state = app.state::<ImportState>();
    let path = PathBuf::from(path);
    // A file that fails leaves no older one to import
    let _ = state.take();
    let checked = read_file(&path).inspect_err(|key| logged("import", &path, key))?;
    let preview = ImportPreview {
        exported_at: checked.exported_at.clone(),
        app_version: checked.app_version.clone(),
        settings: checked.settings.is_some(),
        pages: checked.pages.as_ref().map(Vec::len),
        macros: checked.macros.as_ref().map(Vec::len),
        display: checked.display.is_some(),
        dropped: checked.dropped,
    };
    state.keep(checked);
    Ok(preview)
}

#[tauri::command]
pub fn cancel_import(state: State<ImportState>) {
    let _ = state.take();
}

/// The parts chosen, after a copy of what the app has now; the error is the key of the window's sentence
#[tauri::command(async)]
pub fn apply_import(app: AppHandle, parts: Parts) -> Result<ImportResult, String> {
    let checked = app.state::<ImportState>().take()?;
    let folder = config_folder(&app)?;
    let settings = app.state::<SettingsState>().get();
    let pages = app.state::<CustomPagesState>().get();
    let macros = app.state::<MacrosState>().get();
    let display = device::display_settings(&app);
    write_copy(&folder, &settings, &pages, &macros, display).map_err(|error| {
        logged("keep a copy before importing", &folder, errors::in_english(&error));
        "cannotWrite".to_string()
    })?;
    let merged = merge(&checked, parts, (&settings, &pages, &macros));
    let new = Whole {
        settings: merged.settings,
        pages: merged.pages,
        macros: merged.macros,
    };
    let old = Whole {
        settings,
        pages,
        macros,
    };
    if let Err(error) = put(&app, new, old) {
        drop_copy(&folder);
        return Err(error);
    }
    // Imported: an older copy left in its place would put back what came before the import before
    if let Err(error) = keep_copy(&folder) {
        logged("keep a copy before importing", &folder, errors::in_english(&error));
        let _ = fs::remove_dir_all(folder.join(BEFORE_IMPORT));
    }
    Ok(ImportResult {
        cut: merged.cut,
        display_skipped: !put_display(&app, merged.display),
    })
}

#[tauri::command]
pub fn has_before_import(app: AppHandle) -> bool {
    config_folder(&app).is_ok_and(|folder| self::has_before_import_in(&folder))
}

/// The copy made before the last import, in place of what the import brought; the PC's own settings stay
#[tauri::command(async)]
pub fn restore_before_import(app: AppHandle) -> Result<ImportResult, String> {
    let folder = config_folder(&app)?;
    let before = before_import(&folder).ok_or_else(|| {
        logged("restore", &folder, "the copy cannot be read");
        "cannotRestore".to_string()
    })?;
    let now = Whole {
        settings: app.state::<SettingsState>().get(),
        pages: app.state::<CustomPagesState>().get(),
        macros: app.state::<MacrosState>().get(),
    };
    let new = Whole {
        settings: SharedSettings::of(&before.settings).onto(now.settings.clone()),
        pages: before.pages,
        macros: before.macros,
    };
    put(&app, new, now)?;
    Ok(ImportResult {
        cut: 0,
        display_skipped: !put_display(&app, before.display),
    })
}

/// The three files of a configuration, as the app holds them
struct Whole {
    settings: Settings,
    pages: Vec<CustomPage>,
    macros: Vec<Macro>,
}

enum Part {
    Macros(Vec<Macro>),
    Pages(Vec<CustomPage>),
    Settings(Settings),
}

impl Whole {
    /// Macros first, so that the pages and the knob never point to one not there yet
    fn parts(self) -> [Part; 3] {
        [
            Part::Macros(self.macros),
            Part::Pages(self.pages),
            Part::Settings(self.settings),
        ]
    }
}

/// Each part in turn; when one cannot be saved, every part goes back to `old`, so that the app is never half one
/// configuration and half the other
fn put_all(new: Whole, old: Whole, mut put: impl FnMut(Part) -> Result<(), String>) -> Result<(), String> {
    for part in new.parts() {
        if let Err(error) = put(part) {
            for part in old.parts() {
                let _ = put(part);
            }
            return Err(error);
        }
    }
    Ok(())
}

fn put(app: &AppHandle, new: Whole, old: Whole) -> Result<(), String> {
    let applied = put_all(new, old, |part| match part {
        Part::Macros(macros) => macros::apply(app, macros).map(drop),
        Part::Pages(pages) => custom_pages::apply(app, pages).map(drop),
        Part::Settings(settings) => settings::apply(app, settings).map(drop),
    });
    applied.map_err(|_| "cannotWrite".to_string())
}

/// False when there are display settings for a Drezimo that is not connected
fn put_display(app: &AppHandle, display: Option<DisplaySettings>) -> bool {
    let Some(display) = display else { return true };
    if device::display_settings(app).is_none() {
        return false;
    }
    device::apply_display_settings(app, display);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::custom_pages::CustomPad;
    use crate::knob::TurnAction;
    use crate::macros::Again;
    use crate::press_again::PressAgain;
    use crate::settings::Anchor;
    use crate::soundboard::tests::custom_pad;
    use serde_json::json;

    fn settings() -> Settings {
        Settings {
            language: Language::Italian,
            led_brightness: 70,
            press_again_by_mode: ByMode {
                play_pause: PressAgain::Stop,
                ..ByMode::default()
            },
            knob: KnobSettings {
                turn: TurnAction::SoundboardForOthers,
                double_tap: PressAction::Macro {
                    macro_id: "macro-1".into(),
                },
                ..KnobSettings::default()
            },
            overlay: OverlaySettings {
                anchor: Anchor::Top,
                ..OverlaySettings::default()
            },
            diagnostics: true,
            tour_seen: true,
            mute_for_me_notice_seen: true,
        }
    }

    fn pad(content: Content) -> Option<CustomPad> {
        Some(CustomPad {
            content,
            label: String::new(),
            colour: None,
            press_again: None,
        })
    }

    fn page(id: &str, pads: Vec<Option<CustomPad>>) -> CustomPage {
        let mut pads = pads;
        pads.resize(36, None);
        CustomPage {
            id: id.into(),
            name: "Mine".into(),
            pads,
        }
    }

    fn item(id: &str, steps: Vec<Step>) -> Macro {
        Macro {
            id: id.into(),
            name: "Entrance".into(),
            colour: None,
            again: Again::Stop,
            repeat: 1,
            restore: false,
            steps,
        }
    }

    const DISPLAY: DisplaySettings = DisplaySettings {
        brightness: 40,
        dim_after: 2,
        off_after: 15,
        shift_every: 5,
    };

    fn file(value: Value) -> Vec<u8> {
        value.to_string().into_bytes()
    }

    #[test]
    fn an_export_imports_back_the_same() {
        let settings = settings();
        let sound = Content::Sound {
            id: "s1".into(),
            name: "Honk".into(),
        };
        let pages = vec![page("page-1", vec![pad(sound)])];
        let macros = vec![item("macro-1", vec![Step::StopSounds])];
        let text = export(&settings, &pages, &macros, Some(DISPLAY), "0.2.0", Local::now());
        let checked = check(text.as_bytes()).unwrap();
        assert_eq!(checked.settings, Some(SharedSettings::of(&settings)));
        assert_eq!(checked.pages, Some(pages));
        assert_eq!(checked.macros, Some(macros));
        assert_eq!(checked.display, Some(DISPLAY));
        assert_eq!(checked.app_version, "0.2.0");
        assert_eq!(checked.dropped, Dropped::default());
        let without_board = export(&settings, &[], &[], None, "0.2.0", Local::now());
        assert_eq!(check(without_board.as_bytes()).unwrap().display, None);
    }

    #[test]
    fn per_pc_fields_do_not_travel() {
        let text = export(&settings(), &[], &[], None, "0.2.0", Local::now());
        for field in ["tourSeen", "muteForMeNoticeSeen", "diagnostics"] {
            assert!(!text.contains(field), "{field}");
        }
        let here = Settings::default();
        let imported = SharedSettings::of(&settings()).onto(here.clone());
        assert_eq!(imported.language, Language::Italian);
        assert_eq!(imported.knob.turn, TurnAction::SoundboardForOthers);
        assert_eq!(
            (
                imported.tour_seen,
                imported.mute_for_me_notice_seen,
                imported.diagnostics
            ),
            (here.tour_seen, here.mute_for_me_notice_seen, here.diagnostics)
        );
    }

    #[test]
    fn refuses_what_is_not_a_configuration() {
        assert_eq!(check(&vec![b' '; MAX_BYTES + 1]), Err(Refused::TooBig));
        assert_eq!(check(b"{"), Err(Refused::NotJson));
        assert_eq!(
            check(&file(json!({"drezimo": "x", "format": 1}))),
            Err(Refused::NotDrezimo)
        );
        assert_eq!(check(&file(json!([]))), Err(Refused::NotDrezimo));
        assert_eq!(
            check(&file(json!({"drezimo": "configuration"}))),
            Err(Refused::NotDrezimo)
        );
        assert_eq!(
            check(&file(json!({"drezimo": "configuration", "format": "1"}))),
            Err(Refused::NotDrezimo)
        );
        assert_eq!(
            check(&file(json!({"drezimo": "configuration", "format": 2}))),
            Err(Refused::Newer)
        );
    }

    #[test]
    fn a_broken_part_leaves_the_others_usable() {
        let broken = json!({"drezimo": "configuration", "format": 1, "pages": 7, "macros": [], "settings": "x"});
        let checked = check(&file(broken)).unwrap();
        assert_eq!(checked.pages, None);
        assert_eq!(checked.settings, None);
        assert_eq!(checked.macros, Some(Vec::new()));
        assert_eq!(checked.display, None, "absent, not broken");
        assert_eq!(checked.dropped.parts, 2);
    }

    #[test]
    fn values_out_of_range_come_back_in_range() {
        let wild = json!({"drezimo": "configuration", "format": 1,
            "settings": {"ledBrightness": 500, "language": "klingon", "overlay": {"opacity": 5}},
            "display": {"brightness": 300, "dimAfter": 7, "offAfter": 1000, "shiftEvery": -2}});
        let checked = check(&file(wild)).unwrap();
        let settings = checked.settings.unwrap();
        assert_eq!(settings.led_brightness, 100);
        assert_eq!(settings.language, Language::System);
        assert_eq!(settings.overlay.opacity, 40);
        let display = checked.display.unwrap();
        assert_eq!(
            (
                display.brightness,
                display.dim_after,
                display.off_after,
                display.shift_every
            ),
            (100, 5, 60, 0)
        );
    }

    #[test]
    fn the_apps_own_filters_drop_bad_pages_and_steps() {
        let content = json!({"drezimo": "configuration", "format": 1,
            "pages": [{"id": "bad", "name": "x", "pads": []}, {"id": "page-1", "name": "Mine", "pads": [{"kind": "x"}]}],
            "macros": [{"id": "macro-1", "name": "Entrance", "steps": [{"type": "flying"}, {"type": "stopSounds"}]}]});
        let checked = check(&file(content)).unwrap();
        assert_eq!(checked.pages.unwrap().len(), 1);
        assert_eq!(checked.macros.unwrap()[0].steps, [Step::StopSounds]);
        let dropped = checked.dropped;
        assert_eq!(
            (dropped.pages, dropped.pads, dropped.macros, dropped.steps),
            (1, 1, 0, 1)
        );
    }

    #[test]
    fn an_absurd_file_keeps_only_what_the_filters_allow() {
        let megabyte = "x".repeat(1024 * 1024);
        let mut pages: Vec<Value> = (0..20_000)
            .map(|index| json!({"id": format!("page-{index}"), "name": "P", "pads": []}))
            .collect();
        let long = json!({"kind": "sound", "id": megabyte, "label": megabyte});
        pages.insert(0, json!({"id": "page-big", "name": megabyte, "pads": [long, {"kind": "voice", "id": "v1", "label": megabyte}]}));
        let bytes = file(json!({"drezimo": "configuration", "format": 1, "pages": pages}));
        assert!(bytes.len() < MAX_BYTES);
        let checked = check(&bytes).unwrap();
        let pages = checked.pages.unwrap();
        assert_eq!(pages.len(), 50);
        assert_eq!(pages[0].name.chars().count(), 24);
        assert_eq!(pages[0].pads[0], None, "an id longer than any");
        assert_eq!(pages[0].pads[1].as_ref().unwrap().label.chars().count(), 20);
        assert_eq!(checked.dropped.pads, 1);
        assert_eq!(checked.dropped.pages, 20_001 - 50);
    }

    #[test]
    fn importing_pages_cuts_the_knobs_page_that_goes() {
        let mut current = settings();
        current.knob.double_tap = PressAction::Page { page: "page-9".into() };
        current.knob.long_press = PressAction::Page {
            page: "favourites".into(),
        };
        let pages = [page("page-9", Vec::new())];
        let content = json!({"drezimo": "configuration", "format": 1, "macros": [], "pages": []});
        let checked = check(&file(content)).unwrap();
        let parts = Parts {
            pages: true,
            ..Parts::default()
        };
        let merged = merge(&checked, parts, (&current, &pages, &[]));
        assert_eq!(merged.settings.knob.double_tap, PressAction::Nothing);
        assert_eq!(
            merged.settings.knob.long_press, current.knob.long_press,
            "a page of Voicemod's stays"
        );
        assert_eq!(merged.cut, 1);
        assert_eq!(merged.settings.language, current.language, "settings not imported");
    }

    #[test]
    fn an_import_cuts_only_what_it_breaks() {
        let mut current = settings();
        // Never given a macro, and one deleted since: neither is the import's doing
        current.knob.double_tap = PressAction::Macro { macro_id: "".into() };
        current.knob.long_press = PressAction::Macro {
            macro_id: "macro-old".into(),
        };
        current.knob.tap = PressAction::Page { page: "page-1".into() };
        let dangling = custom_pad(Content::Macro { id: "macro-old".into() });
        let pages = vec![page("page-1", vec![dangling])];
        let content = json!({"drezimo": "configuration", "format": 1, "pages": [],
            "display": {"brightness": 20, "dimAfter": 1, "offAfter": 10, "shiftEvery": 3}});
        let checked = check(&file(content)).unwrap();
        let display_only = Parts {
            display: true,
            ..Parts::default()
        };
        let merged = merge(&checked, display_only, (&current, &pages, &[]));
        assert_eq!((merged.cut, &merged.settings.knob), (0, &current.knob));
        assert_eq!(merged.pages, pages);

        let with_pages = Parts {
            pages: true,
            ..Parts::default()
        };
        let merged = merge(&checked, with_pages, (&current, &pages, &[]));
        assert_eq!(merged.cut, 1, "the tap's page goes");
        assert_eq!(merged.settings.knob.tap, PressAction::Nothing);
        assert_eq!(merged.settings.knob.long_press, current.knob.long_press);
    }

    #[test]
    fn an_unreadable_display_in_the_copy_refuses_the_restore() {
        let folder = folder("display");
        keep(&folder, &settings(), &[], &[], Some(DISPLAY));
        fs::write(folder.join(BEFORE_IMPORT).join(DISPLAY_FILE), "{").unwrap();
        assert_eq!(before_import(&folder), None);
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn importing_pages_without_their_macros_cuts_the_macro_pads() {
        let macro_pad = pad(Content::Macro { id: "macro-7".into() });
        let go_to = pad(Content::Action {
            action: crate::soundboard::Action::GoToPage,
            page: Some("page-3".into()),
        });
        let imported = vec![page("page-1", vec![macro_pad, go_to])];
        let content = json!({"drezimo": "configuration", "format": 1, "pages": imported});
        let checked = check(&file(content)).unwrap();
        let kept = vec![item("macro-1", vec![Step::GoToPage { page: "page-2".into() }])];
        let parts = Parts {
            pages: true,
            ..Parts::default()
        };
        let current = [page("page-2", Vec::new())];
        let merged = merge(&checked, parts, (&settings(), &current, &kept));
        assert_eq!(merged.pages[0].pads[0..2], [None, None]);
        assert!(merged.macros[0].steps.is_empty(), "its page is gone");
        assert_eq!(merged.cut, 3);
        assert_eq!(
            merged.settings.knob.double_tap,
            PressAction::Macro {
                macro_id: "macro-1".into()
            },
            "a macro still there"
        );
    }

    fn folder(name: &str) -> std::path::PathBuf {
        let folder = std::env::temp_dir().join(format!("drezimo-backup-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    fn keep(
        folder: &Path,
        settings: &Settings,
        pages: &[CustomPage],
        macros: &[Macro],
        display: Option<DisplaySettings>,
    ) {
        write_copy(folder, settings, pages, macros, display).unwrap();
        keep_copy(folder).unwrap();
    }

    #[test]
    fn a_failed_import_keeps_the_earlier_copy() {
        let folder = folder("earlier");
        keep(&folder, &settings(), &[], &[], None);
        write_copy(&folder, &Settings::default(), &[], &[], Some(DISPLAY)).unwrap();
        drop_copy(&folder);
        let before = before_import(&folder).unwrap();
        assert_eq!((before.settings, before.display), (settings(), None));
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 1, "nothing left aside");
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn the_before_import_copy_restores_every_file() {
        let folder = folder("copy");
        assert!(!has_before_import_in(&folder));
        let pages = vec![page("page-1", Vec::new())];
        let macros = vec![item("macro-1", vec![Step::StopSounds])];
        keep(&folder, &settings(), &pages, &macros, Some(DISPLAY));
        assert!(has_before_import_in(&folder));
        let before = before_import(&folder).unwrap();
        assert_eq!(
            before,
            Before {
                settings: settings(),
                pages,
                macros,
                display: Some(DISPLAY)
            }
        );

        // The next import keeps what was there then, without Drezimo connected
        keep(&folder, &Settings::default(), &[], &[], None);
        let before = before_import(&folder).unwrap();
        assert_eq!((before.pages.len(), before.display), (0, None));
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_failed_check_writes_nothing() {
        let folder = folder("failed");
        let big = folder.join("big.json");
        fs::write(&big, vec![b' '; MAX_BYTES + 10]).unwrap();
        assert_eq!(read_file(&big).unwrap_err(), "tooBig");
        assert_eq!(read_file(&folder.join("gone.json")).unwrap_err(), "cannotRead");
        let notes = folder.join("notes.json");
        fs::write(&notes, "{\"drezimo\": \"configuration\", \"format\": 9}").unwrap();
        assert_eq!(read_file(&notes).unwrap_err(), "newer");
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 2, "nothing written beside them");
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn apply_without_a_check_is_refused() {
        let state = ImportState::default();
        assert_eq!(state.take().unwrap_err(), "nothingToImport");
        let content = json!({"drezimo": "configuration", "format": 1, "macros": []});
        state.keep(check(&file(content)).unwrap());
        assert!(state.take().is_ok());
        assert_eq!(state.take().unwrap_err(), "nothingToImport", "once");
    }

    #[test]
    fn an_export_replaces_the_file_whole() {
        let folder = folder("export");
        let path = folder.join("drezimo-configuration.json");
        fs::write(&path, "an older one, longer than the new one").unwrap();
        write_atomically(&path, "{}").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{}");
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 1, "nothing left beside it");
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_part_that_cannot_be_saved_puts_every_part_back() {
        let old = Whole {
            settings: Settings::default(),
            pages: Vec::new(),
            macros: Vec::new(),
        };
        let new = Whole {
            settings: settings(),
            pages: vec![page("page-1", Vec::new())],
            macros: vec![item("macro-1", vec![Step::StopSounds])],
        };
        let mut applied = Vec::new();
        let result = put_all(new, old, |part| {
            let (name, imported) = match &part {
                Part::Macros(macros) => ("macros", !macros.is_empty()),
                Part::Pages(pages) => ("pages", !pages.is_empty()),
                Part::Settings(settings) => ("settings", *settings != Settings::default()),
            };
            applied.push((name, imported));
            // Another program holds pages.json a moment
            if name == "pages" && imported {
                Err("held".to_string())
            } else {
                Ok(())
            }
        });
        assert!(result.is_err());
        assert_eq!(
            applied,
            [
                ("macros", true),
                ("pages", true),
                ("macros", false),
                ("pages", false),
                ("settings", false)
            ]
        );
    }
}
