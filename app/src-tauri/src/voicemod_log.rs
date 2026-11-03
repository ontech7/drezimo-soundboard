//! Voicemod's log: the only place that tells when its sounds start and stop (docs/project.md, section 8). The file is
//! followed as it grows and its lines replayed through Voicemod's per-sound rules. The format is internal to Voicemod
//! and may change with an update; without the log, the app predicts playback from the presses alone.

use crate::playback::State;
use crate::soundboard::PlaybackMode;
use chrono::{Local, NaiveDateTime, TimeZone};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

/// Voicemod itself writes the log up to a second late
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const SEARCH_INTERVAL: Duration = Duration::from_secs(5);
/// While Voicemod rotates the log, the file can be missing for a moment
const LOST_AFTER: Duration = Duration::from_secs(2);
/// How much of an existing log is read at start: plenty for the sounds' lengths and for what is playing
const HISTORY_LIMIT: u64 = 4 * 1024 * 1024;
/// Shorter plays are not taken as a sound's length
const MIN_LENGTH: Duration = Duration::from_millis(100);
/// The largest an Ogg page can be: the last one starts within this many bytes of the end
const MAX_OGG_PAGE: u64 = 65_307;

#[derive(Clone, Debug, PartialEq)]
pub struct LogLine {
    pub at: SystemTime,
    pub kind: LineKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LineKind {
    /// Voicemod started
    Run,
    /// A press; `repeat` when the sound was already active, `paused` when it was paused, `length` the length of its
    /// file when readable
    Play {
        id: String,
        mode: PlaybackMode,
        repeat: bool,
        paused: bool,
        length: Option<Duration>,
    },
    /// A sound left Voicemod's active list
    End {
        id: String,
    },
    /// A sound that stops the others is starting
    StopOthers {
        playing: String,
    },
    StopAll,
}

pub enum LogInput {
    /// The log was found: its recent lines
    Opened(Vec<LogLine>),
    Lines(Vec<LogLine>),
    /// The log went away
    Lost,
}

/// Follows Voicemod's log in the background; `on_input` runs on that thread.
pub fn follow(on_input: impl Fn(LogInput) + Send + 'static) {
    thread::spawn(move || {
        let mut log: Option<Log> = None;
        let mut missing_since: Option<Instant> = None;
        loop {
            match &mut log {
                None => match log_path().map(|path| Log::open(&path)) {
                    Some(Ok((opened, history))) => {
                        on_input(LogInput::Opened(history));
                        log = Some(opened);
                    }
                    _ => {
                        thread::sleep(SEARCH_INTERVAL);
                        continue;
                    }
                },
                Some(current) => match current.read_new() {
                    Ok(lines) => {
                        missing_since = None;
                        if !lines.is_empty() {
                            on_input(LogInput::Lines(lines));
                        }
                    }
                    Err(error) => {
                        if missing_since.get_or_insert_with(Instant::now).elapsed() >= LOST_AFTER {
                            crate::errors::log(format!("lost Voicemod's log: {error}"));
                            on_input(LogInput::Lost);
                            log = None;
                            missing_since = None;
                        }
                    }
                },
            }
            thread::sleep(POLL_INTERVAL);
        }
    });
}

/// Where Voicemod keeps its log.
fn log_path() -> Option<PathBuf> {
    let folder = if cfg!(windows) {
        PathBuf::from(std::env::var_os("LOCALAPPDATA")?)
    } else {
        PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support")
    };
    Some(folder.join("VoicemodV3/logs/voicemod-desktop.log")).filter(|path| path.is_file())
}

/// The log file, read from where the previous read stopped.
struct Log {
    path: PathBuf,
    position: u64,
    /// The start of a line still being written
    partial: Vec<u8>,
    /// The lengths of the sound files the log names, each read once
    lengths: HashMap<PathBuf, Option<Duration>>,
}

impl Log {
    fn open(path: &Path) -> io::Result<(Self, Vec<LogLine>)> {
        let start = fs::metadata(path)?.len().saturating_sub(HISTORY_LIMIT);
        let mut log = Self {
            path: path.to_path_buf(),
            position: start,
            partial: Vec::new(),
            lengths: HashMap::new(),
        };
        let mut bytes = log.read_from_position()?;
        if start > 0 {
            // Started inside a line
            let first_line_end = bytes
                .iter()
                .position(|&byte| byte == b'\n')
                .map_or(bytes.len(), |end| end + 1);
            bytes.drain(..first_line_end);
        }
        let history = log.complete_lines(&bytes);
        Ok((log, history))
    }

    /// The lines added since the previous read.
    fn read_new(&mut self) -> io::Result<Vec<LogLine>> {
        let length = fs::metadata(&self.path)?.len();
        if length < self.position {
            // Rotated or emptied: the new file goes on from its start
            self.position = 0;
            self.partial.clear();
        }
        if length == self.position {
            return Ok(Vec::new());
        }
        let bytes = self.read_from_position()?;
        Ok(self.complete_lines(&bytes))
    }

    fn read_from_position(&mut self) -> io::Result<Vec<u8>> {
        // Opened for each read, so the file is never held open while Voicemod rotates it
        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(self.position))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        self.position += bytes.len() as u64;
        Ok(bytes)
    }

    fn complete_lines(&mut self, bytes: &[u8]) -> Vec<LogLine> {
        self.partial.extend_from_slice(bytes);
        let Some(last_end) = self.partial.iter().rposition(|&byte| byte == b'\n') else {
            return Vec::new();
        };
        let complete: Vec<u8> = self.partial.drain(..=last_end).collect();
        String::from_utf8_lossy(&complete)
            .lines()
            .filter_map(|line| parse_line(line, |file| self.file_length(file)))
            .collect()
    }

    fn file_length(&mut self, file: &Path) -> Option<Duration> {
        if let Some(length) = self.lengths.get(file) {
            return *length;
        }
        // A file that cannot be read yet, or is still being written, is tried again at its next play
        let length = ogg_length(file).ok()?;
        self.lengths.insert(file.to_path_buf(), length);
        length
    }
}

/// The length of a plain Ogg Vorbis or Opus file, as Voicemod keeps the Community's sounds; None for other files,
/// such as the soundboards' sounds, which Voicemod keeps obfuscated.
fn ogg_length(path: &Path) -> io::Result<Option<Duration>> {
    let mut file = File::open(path)?;
    let mut head = Vec::new();
    file.by_ref().take(512).read_to_end(&mut head)?;
    if head.len() < 4 {
        return Err(io::ErrorKind::UnexpectedEof.into());
    }
    let Some((rate, skipped)) = ogg_format(&head) else {
        return Ok(None);
    };
    let size = file.seek(SeekFrom::End(0))?;
    file.seek(SeekFrom::Start(size.saturating_sub(MAX_OGG_PAGE)))?;
    let mut tail = Vec::new();
    file.read_to_end(&mut tail)?;
    let granule = last_granule(&tail).ok_or(io::ErrorKind::UnexpectedEof)?;
    let samples = granule.saturating_sub(skipped) as f64;
    Ok(Duration::try_from_secs_f64(samples / f64::from(rate)).ok())
}

/// From the stream's first page: the rate its positions count at, and the samples skipped at the start.
fn ogg_format(head: &[u8]) -> Option<(u32, u64)> {
    if !head.starts_with(b"OggS") {
        return None;
    }
    let packet = head.get(27 + usize::from(*head.get(26)?)..)?;
    if let Some(vorbis) = packet.strip_prefix(b"\x01vorbis") {
        // After the version and the channel count
        let rate = u32::from_le_bytes(vorbis.get(5..9)?.try_into().ok()?);
        Some((rate, 0))
    } else {
        // Opus counts at 48 kHz whatever the source's rate, and skips the encoder's delay
        let opus = packet.strip_prefix(b"OpusHead")?;
        let skipped = u16::from_le_bytes(opus.get(2..4)?.try_into().ok()?);
        Some((48_000, u64::from(skipped)))
    }
}

/// The position at the end of the stream, from its last page.
fn last_granule(tail: &[u8]) -> Option<u64> {
    // Version 0, flagged as the end of the stream
    let start = tail
        .windows(6)
        .rposition(|page| page.starts_with(b"OggS") && page[4] == 0 && page[5] & 4 != 0)?;
    let granule = i64::from_le_bytes(tail.get(start + 6..start + 14)?.try_into().ok()?);
    u64::try_from(granule).ok()
}

/// Voicemod's own sounds, such as the notice its mic mute plays, have ids like this one: they are not the user's
const OWN_SOUND_ID: &str = "00000000-0000-4000-";

/// One line of the log; `file_length` tells the length of a sound file it names.
fn parse_line(line: &str, file_length: impl FnOnce(&Path) -> Option<Duration>) -> Option<LogLine> {
    let at = timestamp(line.get(1..24)?)?;
    let kind = if line.ends_with("------ RUN ------") {
        LineKind::Run
    } else if let Some(rest) = after(line, "playSound ") {
        let (repeat, rest) = match rest.strip_prefix("(processRepMode) ") {
            Some(rest) => (true, rest),
            None => (false, rest),
        };
        let id = rest.strip_prefix("id=")?.split(',').next()?;
        if id.starts_with(OWN_SOUND_ID) {
            return None;
        }
        // The settings follow the sound's path, which could contain anything
        let settings_start = rest.rfind(", repMode=k")?;
        let file = after(&rest[..settings_start], ", path=");
        let settings = &rest[settings_start..];
        LineKind::Play {
            id: id.to_string(),
            mode: PlaybackMode::from_name(value(settings, "repMode=k")?),
            repeat,
            paused: value(settings, "soundStatus=") == Some("kPaused"),
            length: file.and_then(|file| file_length(Path::new(file))),
        }
    } else if let Some(rest) = after(line, "Removes a sound=`") {
        let id = rest.split('`').next()?;
        if id.starts_with(OWN_SOUND_ID) {
            return None;
        }
        LineKind::End { id: id.to_string() }
    } else if let Some(rest) = after(line, "Requested to stop other sounds before playing a sound=") {
        LineKind::StopOthers {
            playing: rest.trim().to_string(),
        }
    } else if line.contains("Requested to stop all sounds") {
        LineKind::StopAll
    } else {
        return None;
    };
    Some(LogLine { at, kind })
}

fn after<'a>(text: &'a str, marker: &str) -> Option<&'a str> {
    text.find(marker).map(|start| &text[start + marker.len()..])
}

/// The value of `key` in Voicemod's `key=value, key=value` settings.
fn value<'a>(settings: &'a str, key: &str) -> Option<&'a str> {
    after(settings, key)?.split(',').next()
}

/// Voicemod writes its local time.
fn timestamp(text: &str) -> Option<SystemTime> {
    let local = NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S%.3f").ok()?;
    // In the hour repeated when the clocks go back, the earlier reading is as good as the later
    Some(Local.from_local_datetime(&local).earliest()?.into())
}

/// The moment of a log line on the app's own clock.
pub fn instant_of(at: SystemTime, now: Instant) -> Instant {
    match SystemTime::now().duration_since(at) {
        Ok(ago) => now.checked_sub(ago).unwrap_or(now),
        Err(ahead) => now + ahead.duration(),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum LogEvent {
    /// `from_start` when the press started the sound, or another instance of it, from the beginning; `length` the
    /// length of its file, when readable
    Played {
        id: String,
        repeat: bool,
        from_start: bool,
        state: State,
        length: Option<Duration>,
    },
    /// `left` is the state of the instances still playing, for a sound with overlapping plays; `length` the sound's
    /// natural length, when this play was not cut short
    Ended {
        id: String,
        left: Option<State>,
        length: Option<Duration>,
    },
    StoppedOthers {
        playing: String,
    },
    StoppedAll,
    /// Voicemod started again: nothing plays
    Restarted,
}

impl LogEvent {
    pub fn sound_id(&self) -> Option<&str> {
        match self {
            Self::Played { id, .. } | Self::Ended { id, .. } => Some(id),
            _ => None,
        }
    }
}

pub struct ActiveSound<'a> {
    pub id: &'a str,
    pub state: State,
    /// When it would have started, had it never been paused
    pub started: SystemTime,
}

struct Tracked {
    started: SystemTime,
    paused_at: Option<SystemTime>,
    state: State,
    count: u32,
    /// Paused, overlapped or stopped by a press: its play time is not its length
    interrupted: bool,
}

/// Voicemod's sounds as its log tells them (verified with tools/voicemod_experiments.py).
#[derive(Default)]
pub struct LogTracker {
    active: HashMap<String, Tracked>,
    as_of: Option<SystemTime>,
}

impl LogTracker {
    pub fn feed(&mut self, line: &LogLine) -> Option<LogEvent> {
        let at = line.at;
        self.as_of = Some(at);
        match &line.kind {
            LineKind::Run => {
                self.active.clear();
                Some(LogEvent::Restarted)
            }
            LineKind::Play {
                id,
                mode,
                repeat,
                paused,
                length,
            } => {
                // A repeat press is logged with the state the sound had before it
                let mut from_start = false;
                match self.active.get_mut(id).filter(|_| *repeat) {
                    None => {
                        from_start = true;
                        let started = Tracked {
                            started: at,
                            paused_at: None,
                            state: State::Playing,
                            count: 1,
                            interrupted: false,
                        };
                        self.active.insert(id.clone(), started);
                    }
                    Some(sound) if *paused => {
                        if let Some(paused_at) = sound.paused_at.take() {
                            sound.started += at.duration_since(paused_at).unwrap_or_default();
                        }
                        sound.state = State::Playing;
                        sound.interrupted = true;
                    }
                    Some(sound) => match mode {
                        PlaybackMode::PlayPause => {
                            sound.state = State::Paused;
                            sound.paused_at = Some(at);
                            sound.interrupted = true;
                        }
                        PlaybackMode::PlayRestart => {
                            from_start = true;
                            sound.started = at;
                        }
                        PlaybackMode::PlayOverlap => {
                            from_start = true;
                            sound.count += 1;
                            sound.interrupted = true;
                        }
                        // A stop: its end line follows
                        _ => sound.interrupted = true,
                    },
                }
                Some(LogEvent::Played {
                    id: id.clone(),
                    repeat: *repeat,
                    from_start,
                    state: self.active[id].state,
                    length: length.filter(|length| *length > MIN_LENGTH),
                })
            }
            LineKind::End { id } => {
                let (left, length) = match self.active.get_mut(id) {
                    None => (None, None),
                    Some(sound) if sound.count > 1 => {
                        sound.count -= 1;
                        (Some(sound.state), None)
                    }
                    Some(_) => {
                        let sound = self.active.remove(id)?;
                        let played = at.duration_since(sound.started).ok();
                        (None, played.filter(|played| !sound.interrupted && *played > MIN_LENGTH))
                    }
                };
                Some(LogEvent::Ended {
                    id: id.clone(),
                    left,
                    length,
                })
            }
            LineKind::StopOthers { playing } => {
                // Real sounds get an end line next; orphans that Voicemod never removes do not
                self.active.retain(|id, _| id == playing);
                Some(LogEvent::StoppedOthers {
                    playing: playing.clone(),
                })
            }
            LineKind::StopAll => {
                self.active.clear();
                Some(LogEvent::StoppedAll)
            }
        }
    }

    pub fn active(&self) -> impl Iterator<Item = ActiveSound<'_>> {
        self.active.iter().map(|(id, sound)| ActiveSound {
            id,
            state: sound.state,
            started: sound.started,
        })
    }

    /// When the last line read was written.
    pub fn as_of(&self) -> Option<SystemTime> {
        self.as_of
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn text(time: &str, message: &str) -> String {
        format!("[2026-11-01 {time}] [AudioAgent] [info] [SoundboardController] {message}")
    }

    fn parse(line: &str) -> Option<LogLine> {
        parse_line(line, |_| None)
    }

    fn line(time: &str, message: &str) -> LogLine {
        parse(&text(time, message)).expect("a sound line")
    }

    fn play(time: &str, id: &str, mode: &str, repeat: bool, status: &str) -> LogLine {
        let press = if repeat {
            "playSound (processRepMode)"
        } else {
            "playSound"
        };
        let message = format!(
            "{press} id={id}, path=/Users/x/Library/Application Support/VoicemodV3/cache/Memes, vol. 2 - 03 - Honk.ogg, \
             repMode=k{mode}, stopOthers=false, engIsUpd=true, soundStatus={status}, soundsCount=1"
        );
        line(time, &message)
    }

    fn end(time: &str, id: &str) -> LogLine {
        line(time, &format!("Removes a sound=`{id}` from active sounds"))
    }

    #[test]
    fn reads_the_sound_lines() {
        let repeat = play("22:37:17.306", "aa3c", "PlayPause", true, "kPaused");
        assert_eq!(
            repeat.kind,
            LineKind::Play {
                id: "aa3c".into(),
                mode: PlaybackMode::PlayPause,
                repeat: true,
                paused: true,
                length: None
            }
        );
        assert_eq!(end("22:37:18.163", "aa3c").kind, LineKind::End { id: "aa3c".into() });
        let stop_others = line(
            "22:37:19.000",
            "Requested to stop other sounds before playing a sound=3cda",
        );
        assert_eq!(stop_others.kind, LineKind::StopOthers { playing: "3cda".into() });
        assert_eq!(
            line("22:37:19.262", "Requested to stop all sounds").kind,
            LineKind::StopAll
        );

        let run = "[2026-10-29 10:32:51.111] [voicemod-desktop] [info] ------ RUN ------";
        assert_eq!(parse(run).unwrap().kind, LineKind::Run);
        assert_eq!(parse(&text("22:37:20.000", "Something else")), None);
        assert_eq!(parse("not a log line"), None);
    }

    #[test]
    fn reads_the_sound_files_lengths() {
        // A Community sound played on Windows: its file is downloaded at the first play, and a first press has no status
        let community = "[2026-11-02 12:12:43.610] [AudioAgent] [info] [SoundboardController] playSound \
            id=2597e302-6b20-4c7a-86bd-373438cf2ee9, \
            path=C:/Users/x/AppData/Local/VoicemodV3/tuna/2597e302-6b20-4c7a-86bd-373438cf2ee9.ogg, \
            repMode=kPlayRestart, stopOthers=true, engIsUpd=true";
        let mut read = None;
        let line = parse_line(community, |file| {
            read = Some(file.to_path_buf());
            Some(Duration::from_millis(1758))
        });
        assert_eq!(
            line.unwrap().kind,
            LineKind::Play {
                id: "2597e302-6b20-4c7a-86bd-373438cf2ee9".into(),
                mode: PlaybackMode::PlayRestart,
                repeat: false,
                paused: false,
                length: Some(Duration::from_millis(1758))
            }
        );
        let file = "C:/Users/x/AppData/Local/VoicemodV3/tuna/2597e302-6b20-4c7a-86bd-373438cf2ee9.ogg";
        assert_eq!(read, Some(PathBuf::from(file)));

        let mut read = None;
        let honk = text(
            "10:00:00.000",
            "playSound id=a, path=/x/cache/Memes, vol. 2 - 03 - Honk.ogg, repMode=kPlayRestart, stopOthers=false",
        );
        parse_line(&honk, |file| {
            read = Some(file.to_path_buf());
            None
        });
        assert_eq!(read, Some(PathBuf::from("/x/cache/Memes, vol. 2 - 03 - Honk.ogg")));
    }

    #[test]
    fn leaves_out_voicemods_own_sounds() {
        let notice = "[2026-11-02 19:07:10.229] [AudioAgent] [info] [SoundboardController] playSound             id=00000000-0000-4000-bcd0-000000000001,             path=C:/Users/x/AppData/Local/VoicemodV3/userCache/resources/MuteMicOn.wav,             repMode=kPlayOverlap, stopOthers=false, engIsUpd=true";
        let removed = "[2026-11-02 19:07:10.904] [AudioAgent] [info] [SoundboardController] Removes a             sound=`00000000-0000-4000-bcd0-000000000001` from active sounds";
        assert_eq!(parse(notice), None);
        assert_eq!(parse(removed), None);
    }

    fn ogg_page(flags: u8, granule: i64, packet: &[u8]) -> Vec<u8> {
        let mut page = b"OggS\0".to_vec();
        page.push(flags);
        page.extend(granule.to_le_bytes());
        // Stream, sequence number and checksum
        page.extend([0; 12]);
        page.extend([1, packet.len() as u8]);
        page.extend(packet);
        page
    }

    #[test]
    fn reads_the_length_of_plain_ogg_files() {
        let vorbis = [
            b"\x01vorbis".as_slice(),
            &0u32.to_le_bytes(),
            &[2],
            &44_100u32.to_le_bytes(),
        ]
        .concat();
        let opus = [
            b"OpusHead".as_slice(),
            &[1, 2],
            &312u16.to_le_bytes(),
            &48_000u32.to_le_bytes(),
        ]
        .concat();
        let cases = [
            (
                "vorbis",
                [
                    ogg_page(2, 0, &vorbis),
                    ogg_page(0, 44_100, &[7; 200]),
                    ogg_page(4, 88_200, &[7; 100]),
                ]
                .concat(),
                Some(Duration::from_secs(2)),
            ),
            (
                "opus",
                [ogg_page(2, 0, &opus), ogg_page(4, 48_312, &[7; 100])].concat(),
                Some(Duration::from_secs(1)),
            ),
            ("obfuscated", b"}TP5dijsfhd4zx\"\x08".to_vec(), None),
        ];
        for (name, bytes, length) in cases {
            let path = std::env::temp_dir().join(format!("drezimo-{name}-{}.ogg", std::process::id()));
            fs::write(&path, bytes).unwrap();
            let read = ogg_length(&path).unwrap();
            fs::remove_file(&path).unwrap();
            assert_eq!(read, length, "{name}");
        }
    }

    #[test]
    fn times_are_exact_to_the_millisecond() {
        let start = play("23:59:59.400", "a", "PlayRestart", false, "N/A").at;
        let end = end("23:59:59.900", "a").at;
        assert_eq!(end.duration_since(start).unwrap(), Duration::from_millis(500));
    }

    #[test]
    fn learns_the_length_of_uninterrupted_plays() {
        let mut tracker = LogTracker::default();
        tracker.feed(&play("10:00:00.000", "a", "PlayRestart", false, "N/A"));
        assert_eq!(tracker.active().count(), 1);
        let ended = tracker.feed(&end("10:00:00.850", "a"));
        let length = Some(Duration::from_millis(850));
        assert!(matches!(ended, Some(LogEvent::Ended { length: learned, .. }) if learned == length));
    }

    #[test]
    fn a_pause_is_neither_play_time_nor_length() {
        let mut tracker = LogTracker::default();
        tracker.feed(&play("10:00:01.000", "p", "PlayPause", false, "N/A"));
        let paused = tracker.feed(&play("10:00:02.000", "p", "PlayPause", true, "kPlaying"));
        assert!(matches!(
            paused,
            Some(LogEvent::Played {
                state: State::Paused,
                ..
            })
        ));

        tracker.feed(&play("10:00:09.000", "p", "PlayPause", true, "kPaused"));
        let resumed = tracker.active().next().unwrap();
        assert_eq!(resumed.started, play("10:00:08.000", "x", "PlayPause", false, "N/A").at);

        let ended = tracker.feed(&end("10:00:10.000", "p"));
        assert!(matches!(ended, Some(LogEvent::Ended { length: None, .. })));
    }

    #[test]
    fn tells_presses_that_start_from_the_beginning() {
        let mut tracker = LogTracker::default();
        let cases = [
            (play("10:00:00.000", "a", "PlayRestart", false, "N/A"), true),
            (play("10:00:01.000", "a", "PlayRestart", true, "kPlaying"), true),
            (play("10:00:02.000", "b", "PlayStop", false, "N/A"), true),
            (play("10:00:03.000", "b", "PlayStop", true, "kPlaying"), false),
        ];
        for (line, expected) in cases {
            let event = tracker.feed(&line);
            assert!(
                matches!(event, Some(LogEvent::Played { from_start, .. }) if from_start == expected),
                "{line:?}"
            );
        }
    }

    #[test]
    fn stops_clear_sounds_even_without_their_end_line() {
        let mut tracker = LogTracker::default();
        tracker.feed(&play("10:00:00.000", "orphan", "PlayRestart", false, "N/A"));
        tracker.feed(&play("10:00:01.000", "loop", "PlayStop", false, "N/A"));
        let stop_others = line(
            "10:00:02.000",
            "Requested to stop other sounds before playing a sound=loop",
        );
        let playing = "loop".to_string();
        assert_eq!(tracker.feed(&stop_others), Some(LogEvent::StoppedOthers { playing }));
        let ids: Vec<&str> = tracker.active().map(|sound| sound.id).collect();
        assert_eq!(ids, ["loop"]);

        let stop_all = line("10:00:03.000", "Requested to stop all sounds");
        assert_eq!(tracker.feed(&stop_all), Some(LogEvent::StoppedAll));
        assert_eq!(tracker.active().count(), 0);

        tracker.feed(&play("10:00:04.000", "loop", "PlayStop", false, "N/A"));
        let run = parse("[2026-11-01 10:00:05.000] [voicemod-desktop] [info] ------ RUN ------").unwrap();
        assert_eq!(tracker.feed(&run), Some(LogEvent::Restarted));
        assert_eq!(tracker.active().count(), 0, "a restarted Voicemod plays nothing");
    }

    #[test]
    fn counts_overlapping_plays_without_learning_their_length() {
        let mut tracker = LogTracker::default();
        tracker.feed(&play("10:00:00.000", "o", "PlayOverlap", false, "N/A"));
        tracker.feed(&play("10:00:00.500", "o", "PlayOverlap", true, "kPlaying"));
        let first_end = tracker.feed(&end("10:00:01.000", "o"));
        assert!(matches!(
            first_end,
            Some(LogEvent::Ended {
                left: Some(State::Playing),
                ..
            })
        ));
        let last_end = tracker.feed(&end("10:00:01.500", "o"));
        assert!(matches!(
            last_end,
            Some(LogEvent::Ended {
                left: None,
                length: None,
                ..
            })
        ));
    }

    #[test]
    fn follows_the_file_as_it_grows_and_rotates() {
        let path = std::env::temp_dir().join(format!("drezimo-log-test-{}.log", std::process::id()));
        let honk = text("10:00:00.000", "Removes a sound=`honk` from active sounds");
        let half = text("10:00:01.000", "Removes a sound=`half` from active sounds");
        let (first_half, second_half) = half.split_at(30);
        fs::write(&path, format!("{honk}\r\nnoise\n{first_half}")).unwrap();

        let (mut log, history) = Log::open(&path).unwrap();
        assert_eq!(history.len(), 1);
        assert!(log.read_new().unwrap().is_empty());

        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(file, "{second_half}").unwrap();
        assert_eq!(log.read_new().unwrap()[0].kind, LineKind::End { id: "half".into() });

        fs::write(&path, format!("{honk}\n")).unwrap();
        assert_eq!(log.read_new().unwrap()[0].kind, LineKind::End { id: "honk".into() });
        fs::remove_file(&path).unwrap();
        assert!(log.read_new().is_err());
    }
}
