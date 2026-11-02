//! The board's firmware, built into the app. A board with an older one is updated as soon as it connects, through the
//! RP2040's own USB bootloader (docs/project.md, section 9): after `BOOT` the board shows up as a drive, RPI-RP2, that
//! takes the firmware as a UF2 file and restarts with it. Nothing changes on the board until the whole file is written.

use serde::Serialize;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

pub const VERSION: &str = env!("DREZIMO_FIRMWARE_VERSION");
const IMAGE: &[u8] = include_bytes!(env!("DREZIMO_FIRMWARE_IMAGE"));
/// How long the board may take to show up as a drive after `BOOT`
const BOOTLOADER_TIMEOUT: Duration = Duration::from_secs(30);
/// How long the board may take to come back once its firmware is written: a few seconds are enough
const RESTART_TIMEOUT: Duration = Duration::from_secs(15);
const POLL: Duration = Duration::from_millis(250);

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum FirmwareUpdate {
    /// Sending the new firmware to the board
    Updating {
        from: String,
        to: String,
    },
    /// Written: waiting for the board to start with it
    Restarting {
        to: String,
    },
    Failed {
        reason: String,
    },
}

/// Decides when the board gets the app's firmware. A board that keeps its old firmware is tried once, so it is not sent
/// back to its bootloader again and again.
#[derive(Default)]
pub struct Updater {
    tried: bool,
    state: Option<FirmwareUpdate>,
    /// When a board written to must be back
    deadline: Option<Instant>,
}

impl Updater {
    /// True when the board, just connected with `version`, must be updated now.
    pub fn board_connected(&mut self, version: &str) -> bool {
        self.deadline = None;
        if !is_outdated(version) {
            *self = Self::default();
            return false;
        }
        if !self.tried {
            self.tried = true;
            self.state = Some(FirmwareUpdate::Updating {
                from: version.to_string(),
                to: VERSION.to_string(),
            });
            return true;
        }
        if !matches!(self.state, Some(FirmwareUpdate::Failed { .. })) {
            self.state = Some(FirmwareUpdate::Failed {
                reason: format!("The board kept its firmware {version}."),
            });
        }
        false
    }

    /// Takes the result of `install`.
    pub fn installed(&mut self, result: Result<(), String>, now: Instant) {
        // A board already back with the new firmware has finished the update
        if !matches!(self.state, Some(FirmwareUpdate::Updating { .. })) {
            return;
        }
        self.state = Some(match result {
            Ok(()) => {
                self.deadline = Some(now + RESTART_TIMEOUT);
                FirmwareUpdate::Restarting {
                    to: VERSION.to_string(),
                }
            }
            Err(reason) => FirmwareUpdate::Failed { reason },
        });
    }

    /// Applies what is due by `now`: a board that did not come back is reported.
    pub fn update(&mut self, now: Instant) {
        if self.deadline.is_some_and(|deadline| deadline <= now) {
            self.deadline = None;
            self.state = Some(FirmwareUpdate::Failed {
                reason: "The board did not come back with the new firmware. Unplug it and plug it back in.".to_string(),
            });
        }
    }

    pub fn next_update(&self) -> Option<Instant> {
        self.deadline
    }

    pub fn state(&self) -> Option<FirmwareUpdate> {
        self.state.clone()
    }
}

/// Whether a board reporting `version` has an older firmware than the app's.
pub fn is_outdated(version: &str) -> bool {
    parse(version)
        .zip(parse(VERSION))
        .is_some_and(|(board, app)| board < app)
}

/// "1.2.3", or "1.2.3-test", as numbers that sort a test build before its release.
fn parse(version: &str) -> Option<(u32, u32, u32, bool)> {
    let (numbers, suffix) = version.split_once('-').unwrap_or((version, ""));
    let mut numbers = numbers.split('.').map(|part| part.parse().ok());
    let parsed = (numbers.next()??, numbers.next()??, numbers.next()??, suffix.is_empty());
    numbers.next().is_none().then_some(parsed)
}

/// The RP2040 bootloader drives present now, told apart by the file each one carries.
pub fn bootloader_drives() -> Vec<PathBuf> {
    without_disk_prompts(|| {
        drives()
            .into_iter()
            .filter(|drive| {
                fs::read_to_string(drive.join("INFO_UF2.TXT")).is_ok_and(|info| info.contains("Board-ID: RPI-RP2"))
            })
            .collect()
    })
}

/// Waits for the bootloader drive that `BOOT` brings up, one not in `before`, and writes the firmware to it. Another
/// RP2040 board already waiting in its bootloader is left alone.
pub fn install(before: &[PathBuf]) -> Result<(), String> {
    let deadline = Instant::now() + BOOTLOADER_TIMEOUT;
    let drive = loop {
        if let Some(drive) = bootloader_drives().into_iter().find(|drive| !before.contains(drive)) {
            break drive;
        }
        if Instant::now() >= deadline {
            return Err("The board's update drive did not appear. Unplug the board and plug it back in.".to_string());
        }
        thread::sleep(POLL);
    };

    let written = File::create(drive.join("DREZIMO.UF2")).and_then(|mut file| {
        file.write_all(IMAGE)?;
        file.sync_all()
    });
    let Err(error) = written else {
        return Ok(());
    };
    // The board restarts as soon as it has the last block: its drive can go before the file is closed
    let gone_by = Instant::now() + Duration::from_secs(2);
    while Instant::now() < gone_by {
        if !drive.exists() {
            return Ok(());
        }
        thread::sleep(POLL);
    }
    Err(format!("Cannot write the firmware to the board: {error}."))
}

/// The volumes named after the bootloader's drive: others, such as network shares, can be slow to answer.
#[cfg(target_os = "macos")]
fn drives() -> Vec<PathBuf> {
    fs::read_dir("/Volumes")
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.file_name().to_string_lossy().starts_with("RPI-RP2"))
                .map(|entry| entry.path())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(target_os = "macos")]
fn without_disk_prompts<T>(look: impl FnOnce() -> T) -> T {
    look()
}

#[cfg(windows)]
fn drives() -> Vec<PathBuf> {
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};
    use windows_sys::Win32::System::WindowsProgramming::DRIVE_REMOVABLE;

    let present = unsafe { GetLogicalDrives() };
    (0..26u8)
        .filter(|letter| present & (1 << letter) != 0)
        .map(|letter| format!("{}:\\", char::from(b'A' + letter)))
        .filter(|root| {
            let wide: Vec<u16> = root.encode_utf16().chain([0]).collect();
            unsafe { GetDriveTypeW(wide.as_ptr()) == DRIVE_REMOVABLE }
        })
        .map(PathBuf::from)
        .collect()
}

/// Runs `look` with an empty card reader answering with an error, rather than asking for a disk.
#[cfg(windows)]
fn without_disk_prompts<T>(look: impl FnOnce() -> T) -> T {
    use windows_sys::Win32::System::Diagnostics::Debug::{SEM_FAILCRITICALERRORS, SetThreadErrorMode};

    let mut previous = 0;
    unsafe { SetThreadErrorMode(SEM_FAILCRITICALERRORS, &mut previous) };
    let result = look();
    unsafe { SetThreadErrorMode(previous, std::ptr::null_mut()) };
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A version older than the app's
    fn older() -> String {
        let (major, minor, patch, _) = parse(VERSION).unwrap();
        match (major, minor, patch) {
            (_, _, 1..) => format!("{major}.{minor}.{}", patch - 1),
            (_, 1.., 0) => format!("{major}.{}.9", minor - 1),
            _ => format!("{}.9.9", major - 1),
        }
    }

    #[test]
    fn compares_versions_as_numbers() {
        assert!(older() != VERSION && is_outdated(&older()));
        assert!(!is_outdated(VERSION));
        assert!(!is_outdated("99.0.0"));
        assert!(!is_outdated("unknown"), "an unknown firmware is left alone");
        assert!(!is_outdated("1.2"), "an unknown firmware is left alone");
        assert!(parse("0.9.9") < parse("0.10.0"));
        assert!(
            is_outdated(&format!("{VERSION}-test")),
            "a test build gives way to its release"
        );
        assert!(!is_outdated("99.0.0-test"));
    }

    #[test]
    fn updates_an_older_board_once() {
        let mut updater = Updater::default();
        let start = Instant::now();
        assert!(updater.board_connected(&older()));
        assert!(matches!(updater.state(), Some(FirmwareUpdate::Updating { .. })));
        updater.installed(Ok(()), start);
        assert!(matches!(updater.state(), Some(FirmwareUpdate::Restarting { .. })));

        assert!(!updater.board_connected(VERSION));
        assert_eq!(updater.state(), None);
        assert_eq!(updater.next_update(), None);
    }

    #[test]
    fn a_board_that_keeps_its_old_firmware_is_not_tried_again() {
        let mut updater = Updater::default();
        assert!(updater.board_connected(&older()));
        updater.installed(Ok(()), Instant::now());
        assert!(!updater.board_connected(&older()));
        assert!(matches!(updater.state(), Some(FirmwareUpdate::Failed { .. })));
        assert!(!updater.board_connected(&older()));
    }

    #[test]
    fn reports_a_board_that_does_not_come_back() {
        let mut updater = Updater::default();
        let start = Instant::now();
        updater.board_connected(&older());
        updater.installed(Ok(()), start);
        let deadline = updater.next_update().unwrap();
        updater.update(deadline - Duration::from_millis(1));
        assert!(matches!(updater.state(), Some(FirmwareUpdate::Restarting { .. })));
        updater.update(deadline);
        assert!(matches!(updater.state(), Some(FirmwareUpdate::Failed { .. })));
    }

    #[test]
    fn a_board_back_before_the_write_is_reported_stays_updated() {
        let mut updater = Updater::default();
        updater.board_connected(&older());
        updater.board_connected(VERSION);
        updater.installed(Ok(()), Instant::now());
        assert_eq!((updater.state(), updater.next_update()), (None, None));
    }

    #[test]
    fn a_failed_write_is_reported() {
        let mut updater = Updater::default();
        updater.board_connected(&older());
        updater.installed(
            Err("The board did not switch to its update mode.".into()),
            Instant::now(),
        );
        assert!(matches!(updater.state(), Some(FirmwareUpdate::Failed { .. })));
    }

    #[test]
    fn the_bundled_firmware_is_a_uf2_file() {
        // Each 512-byte block starts with the UF2 magic numbers
        assert_eq!(IMAGE.len() % 512, 0);
        assert_eq!(&IMAGE[..8], &[0x55, 0x46, 0x32, 0x0A, 0x57, 0x51, 0x5D, 0x9E]);
    }
}
