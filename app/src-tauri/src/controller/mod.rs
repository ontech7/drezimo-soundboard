//! What the device does, as in the simulator: the pads play the sounds of the current page, the function keys and
//! the knob drive Voicemod and the pages, and the LEDs and the display show what Voicemod is doing. Plain logic: it
//! gets each input with the time it happened and queues the messages for the board and the requests for Voicemod.

mod screen;

use crate::display::Frame;
use crate::display::screens::{self, Screen};
use crate::display::texts::{ENGLISH, Texts};
use crate::playback::{LoggedSound, Playback, State};
use crate::protocol::{BoardMessage, Effect, LED_COUNT, Led, PAD_COUNT};
use crate::soundboard::{self, Page, PlaybackMode, Sound, Soundboard};
use crate::voicemod::{Request, VoicemodEvent, VoicemodStatus};
use crate::voicemod_log::{self, LogEvent, LogInput, LogLine, LogTracker};
use serde::Serialize;
use std::collections::HashMap;
use std::mem;
use std::time::{Duration, Instant};

const PADS: usize = PAD_COUNT as usize;
/// How long a pressed pad shows yellow
const PRESS_FLASH: Duration = Duration::from_millis(200);
/// A key-up sent right after the key-down can leave a sound stuck in Voicemod (docs/project.md, section 8)
const MIN_HOLD: Duration = Duration::from_millis(200);
/// How long the soundboard menu stays open after the knob last moved
const MENU_TIME: Duration = Duration::from_secs(6);
/// How long a peeked name stays after the press
const PEEK_TIME: Duration = Duration::from_millis(2500);
const ROWS: usize = 6;
const COLUMNS: usize = 6;

// Function keys, from the left
const PREV: u8 = 0;
const NEXT: u8 = 1;
const STOP: u8 = 2;
const MUTE_ME: u8 = 3;
const VOICE: u8 = 4;
const HEAR_ME: u8 = 5;

// The simulator's colours
const BLUE: [u8; 3] = [47, 107, 255];
const YELLOW: [u8; 3] = [255, 210, 26];
const GREEN: [u8; 3] = [34, 224, 90];
const DIM_RED: [u8; 3] = [90, 16, 16];
const RED: [u8; 3] = [255, 42, 42];
const WHITE: [u8; 3] = [120, 120, 130];
const AMBER: [u8; 3] = [255, 140, 0];
/// The peeked pad and the row the overview shows
const HIGHLIGHT: [u8; 3] = [200, 200, 210];

#[derive(Debug, PartialEq)]
pub enum Output {
    Board(String),
    Voicemod(Request),
    /// A new picture for the display, whether or not a board shows it
    Display(Frame),
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageSummary {
    pub name: String,
    pub part: usize,
    pub parts: usize,
    pub number: usize,
    pub count: usize,
}

/// A held pad whose sound plays only while held: it gets a key-up on release
struct HeldSound {
    sound: Sound,
    since: Instant,
}

#[derive(Default)]
struct Knob {
    pressed: bool,
    /// Turned, or held for a peek, since it was pressed: its release is not a tap
    used: bool,
    /// The row the overview shows, while the knob is held and turned
    row: Option<usize>,
}

struct Menu {
    selected: usize,
    until: Instant,
}

struct Peek {
    pad: u8,
    until: Instant,
}

#[derive(Default)]
struct Switches {
    mute_for_me: bool,
    voice_changer: bool,
    hear_myself: bool,
}

pub struct Controller {
    texts: &'static Texts,
    voicemod: VoicemodStatus,
    switches: Switches,
    pages: Vec<Page>,
    page: usize,
    /// The page to show again when the soundboards come back after Voicemod was away: soundboard and part
    resume: Option<(String, usize)>,
    playback: Playback,
    /// Voicemod's sounds as its log tells them
    log: LogTracker,
    /// Every sound of every soundboard, by id
    sounds: HashMap<String, Sound>,
    /// The pads held down. The sound is kept, so its key-up still reaches it after a page change.
    held: HashMap<u8, Option<HeldSound>>,
    /// Until when each pressed pad shows yellow
    flashes: HashMap<u8, Instant>,
    /// Key-ups waiting for the minimum hold, and when they are due
    releases: Vec<(Instant, HeldSound)>,
    knob: Knob,
    menu: Option<Menu>,
    peek: Option<Peek>,
    /// The name of the last sound played, or of the last stop, and when
    last: Option<(String, Instant)>,
    board_connected: bool,
    /// What the board's LEDs show, when known
    leds: [Option<Led>; LED_COUNT],
    /// What the display shows, when sent
    screen: Option<Screen>,
    /// When `update` has something to do without any input
    next_update: Option<Instant>,
    outputs: Vec<Output>,
}

impl Default for Controller {
    fn default() -> Self {
        Self {
            texts: &ENGLISH,
            voicemod: VoicemodStatus::default(),
            switches: Switches::default(),
            pages: Vec::new(),
            page: 0,
            resume: None,
            playback: Playback::default(),
            log: LogTracker::default(),
            sounds: HashMap::new(),
            held: HashMap::new(),
            flashes: HashMap::new(),
            releases: Vec::new(),
            knob: Knob::default(),
            menu: None,
            peek: None,
            last: None,
            board_connected: false,
            leds: [None; LED_COUNT],
            screen: None,
            next_update: None,
            outputs: Vec::new(),
        }
    }
}

impl Controller {
    pub fn new(texts: &'static Texts) -> Self {
        Self {
            texts,
            ..Self::default()
        }
    }

    pub fn set_board_connected(&mut self, connected: bool, now: Instant) {
        if connected == self.board_connected {
            return;
        }
        self.board_connected = connected;
        // A board that connects waits for every LED and a picture; one that went away cannot report its keys' release
        self.leds = [None; LED_COUNT];
        self.screen = None;
        self.knob = Knob::default();
        if !connected {
            let held: Vec<u8> = self.held.keys().copied().collect();
            for pad in held {
                self.pad_released(pad, now);
            }
        }
    }

    pub fn board_message(&mut self, message: BoardMessage, now: Instant) {
        match message {
            BoardMessage::Pad { index, pressed: true } => self.pad_pressed(index, now),
            BoardMessage::Pad { index, pressed: false } => self.pad_released(index, now),
            BoardMessage::Button { index, pressed: true } => self.button_pressed(index, now),
            BoardMessage::Encoder { step } => self.knob_turned(step.into(), now),
            BoardMessage::EncoderButton { pressed } => self.knob_pressed(pressed, now),
            _ => {}
        }
    }

    pub fn voicemod_event(&mut self, event: VoicemodEvent, now: Instant) {
        match event {
            VoicemodEvent::Status(status) => {
                self.voicemod = status;
                // The waiting key-ups can no longer reach what played, and the soundboards may change meanwhile:
                // Voicemod sends them again once connected. What plays then is in the log, if followed.
                self.playback.reset();
                if status == VoicemodStatus::Connected {
                    self.resync(now);
                }
                self.releases.clear();
                self.switches = Switches::default();
                if let Some(page) = self.page_id(self.page) {
                    self.resume = Some(page);
                }
                self.pages.clear();
                self.page = 0;
                self.menu = None;
            }
            VoicemodEvent::Soundboards(soundboards) => self.set_soundboards(&soundboards),
            VoicemodEvent::ActiveSoundboard(id) => {
                if let Some(page) = self.pages.iter().position(|page| page.soundboard_id == id) {
                    self.page = page;
                }
            }
            VoicemodEvent::MuteForMe(on) => self.switches.mute_for_me = on,
            VoicemodEvent::VoiceChanger(on) => self.switches.voice_changer = on,
            VoicemodEvent::HearMyself(on) => self.switches.hear_myself = on,
        }
    }

    /// Applies what is due by `now` and queues the changes for the LEDs and the display.
    pub fn update(&mut self, now: Instant) {
        let (due, waiting): (Vec<_>, Vec<_>) = mem::take(&mut self.releases)
            .into_iter()
            .partition(|(at, _)| *at <= now);
        self.releases = waiting;
        for (_, HeldSound { sound, .. }) in due {
            // Not to a sound stopped meanwhile: Voicemod would take the key-up as a press and start it again
            if self.playback.state(&sound.id).is_some() {
                self.playback.release(&sound, now);
                self.request(Request::PlaySound {
                    id: sound.id,
                    key_down: false,
                });
            }
        }
        self.flashes.retain(|_, until| *until > now);
        if self.playback.update(now) {
            self.resync(now);
        }
        self.menu = self.menu.take().filter(|menu| menu.until > now);
        // A peek lasts while the knob is held, its blink only its time
        self.peek = self.peek.take().filter(|peek| peek.until > now || self.knob.pressed);
        self.last = self.last.take().filter(|(_, at)| now < *at + screen::LAST_SOUND_TIME);
        if self.board_connected {
            self.refresh_leds(now);
        }

        let (screen, screen_changes) = self.screen(now);
        if self.screen.as_ref() != Some(&screen) {
            self.outputs.push(Output::Display(screens::draw(&screen).frame()));
            self.screen = Some(screen);
        }

        let flashes = self.flashes.values().copied();
        let releases = self.releases.iter().map(|(at, _)| *at);
        let menu_end = self.menu.as_ref().map(|menu| menu.until);
        // A peek held past its time only waits for the knob
        let peek_end = self.peek.as_ref().map(|peek| peek.until).filter(|until| *until > now);
        self.next_update = flashes
            .chain(releases)
            .chain(self.playback.next_change())
            .chain(menu_end)
            .chain(peek_end)
            .chain(screen_changes)
            .min();
    }

    /// When `update` has something to do without any input.
    pub fn next_update(&self) -> Option<Instant> {
        self.next_update
    }

    pub fn log_input(&mut self, input: LogInput, now: Instant) {
        match input {
            LogInput::Opened(history) => {
                // The past only teaches the sounds' lengths and what is playing now
                self.log = LogTracker::default();
                for line in &history {
                    if let Some(LogEvent::Ended {
                        id,
                        length: Some(length),
                        ..
                    }) = self.log.feed(line)
                    {
                        self.playback.learn_length(&id, length);
                    }
                }
                self.playback.follow_log(true);
                self.resync(now);
            }
            LogInput::Lines(lines) => {
                let mut started_elsewhere = false;
                for line in &lines {
                    started_elsewhere |= self.log_line(line, now);
                }
                // A sound started from the computer brightens a dimmed display, as a key would
                if started_elsewhere && self.board_connected {
                    self.outputs.push(Output::Board("WAKE".to_string()));
                }
            }
            LogInput::Lost => self.playback.follow_log(false),
        }
    }

    /// Whether the playback shown comes from Voicemod's log, rather than from guesses.
    pub fn follows_log(&self) -> bool {
        self.playback.follows_log()
    }

    pub fn take_outputs(&mut self) -> Vec<Output> {
        mem::take(&mut self.outputs)
    }

    pub fn page(&self) -> Option<PageSummary> {
        let page = self.pages.get(self.page)?;
        Some(PageSummary {
            name: page.name.clone(),
            part: page.part,
            parts: page.parts,
            number: self.page + 1,
            count: self.pages.len(),
        })
    }

    fn pad_pressed(&mut self, pad: u8, now: Instant) {
        if self.held.contains_key(&pad) {
            return;
        }
        if self.knob.pressed {
            // A peek: the name shows, nothing plays
            self.knob.used = true;
            self.held.insert(pad, None);
            self.peek = Some(Peek {
                pad,
                until: now + PEEK_TIME,
            });
            return;
        }
        self.flashes.insert(pad, now + PRESS_FLASH);
        let waiting = self
            .sound_at(pad)
            .and_then(|sound| self.releases.iter().position(|(_, held)| held.sound.id == sound.id));
        if let Some(index) = waiting {
            let (_, held) = self.releases.remove(index);
            // Pressed again before its key-up went out: for Voicemod, the key was simply held all along
            if self.playback.state(&held.sound.id).is_some() {
                self.held.insert(pad, Some(held));
                return;
            }
        }

        let playable = self.sound_at(pad).filter(|sound| sound.enabled).cloned();
        let Some(sound) = playable.filter(|_| self.voicemod == VoicemodStatus::Connected) else {
            self.held.insert(pad, None);
            return;
        };

        self.request(Request::PlaySound {
            id: sound.id.clone(),
            key_down: true,
        });
        self.playback.press(&sound, now);
        self.last = Some((sound.name.clone(), now));
        let held_sound =
            (sound.playback_mode == PlaybackMode::PlayLoopOnPress).then_some(HeldSound { sound, since: now });
        self.held.insert(pad, held_sound);
    }

    fn pad_released(&mut self, pad: u8, now: Instant) {
        // Voicemod takes every request as a press: only the sounds that play while held get the key-up
        if let Some(Some(held)) = self.held.remove(&pad) {
            self.releases.push((now.max(held.since + MIN_HOLD), held));
        }
    }

    fn button_pressed(&mut self, button: u8, now: Instant) {
        match button {
            PREV => self.turn_page(-1),
            NEXT => self.turn_page(1),
            STOP => {
                self.playback.stop_all(now);
                self.request(Request::StopAllSounds);
                self.last = Some((self.texts.stop.to_string(), now));
            }
            MUTE_ME => self.request(Request::ToggleMuteForMe),
            VOICE => self.request(Request::ToggleVoiceChanger),
            HEAR_ME => self.request(Request::ToggleHearMyself),
            _ => {}
        }
    }

    fn knob_turned(&mut self, step: isize, now: Instant) {
        if self.knob.pressed {
            self.knob.used = true;
            self.knob.row = Some(match self.knob.row {
                None if step > 0 => 0,
                None => ROWS - 1,
                Some(row) => (row as isize + step).rem_euclid(ROWS as isize) as usize,
            });
            self.peek = None;
        } else if let Some(menu) = &mut self.menu {
            let last = self.pages.len().saturating_sub(1) as isize;
            menu.selected = (menu.selected as isize + step).clamp(0, last) as usize;
            menu.until = now + MENU_TIME;
        } else {
            self.turn_page(step);
        }
    }

    /// A tap opens the soundboard menu, and another picks the page selected.
    fn knob_pressed(&mut self, pressed: bool, now: Instant) {
        if pressed == self.knob.pressed {
            return;
        }
        let tapped = !pressed && !self.knob.used;
        self.knob = Knob {
            pressed,
            ..Knob::default()
        };
        if !tapped {
            return;
        }
        match self.menu.take() {
            Some(menu) => self.page = menu.selected.min(self.pages.len().saturating_sub(1)),
            None => {
                self.menu = Some(Menu {
                    selected: self.page,
                    until: now + MENU_TIME,
                })
            }
        }
    }

    fn turn_page(&mut self, step: isize) {
        let count = self.pages.len() as isize;
        if count > 0 {
            self.page = (self.page as isize + step).rem_euclid(count) as usize;
        }
    }

    fn set_soundboards(&mut self, soundboards: &[Soundboard]) {
        let current = self.resume.take().or_else(|| self.page_id(self.page));
        let selected = self.menu.as_ref().and_then(|menu| self.page_id(menu.selected));
        self.sounds = soundboards
            .iter()
            .flat_map(|soundboard| &soundboard.sounds)
            .map(|sound| (sound.id.clone(), sound.clone()))
            .collect();
        self.pages = soundboard::pages(soundboards);
        self.page = current.and_then(|id| self.find_page(&id)).unwrap_or(0);
        let remapped = selected.and_then(|id| self.find_page(&id));
        let last = self.pages.len().saturating_sub(1);
        if let Some(menu) = &mut self.menu {
            menu.selected = remapped.unwrap_or(menu.selected.min(last));
        }
    }

    /// The page of a soundboard and part, after the soundboards changed.
    fn find_page(&self, (id, part): &(String, usize)) -> Option<usize> {
        let same_soundboard = |page: &Page| page.soundboard_id == *id;
        let exact = self
            .pages
            .iter()
            .position(|page| same_soundboard(page) && page.part == *part);
        // A soundboard that shrank keeps its last page
        exact.or_else(|| self.pages.iter().rposition(same_soundboard))
    }

    /// True when the line tells of a sound started from the computer.
    fn log_line(&mut self, line: &LogLine, now: Instant) -> bool {
        let Some(event) = self.log.feed(line) else {
            return false;
        };
        if let LogEvent::Ended {
            id,
            length: Some(length),
            ..
        } = &event
        {
            self.playback.learn_length(id, *length);
        }
        // A log no longer trusted only teaches lengths
        if self.voicemod != VoicemodStatus::Connected || !self.playback.follows_log() {
            return false;
        }
        let sound = event.sound_id().and_then(|id| self.sounds.get(id));
        let unpredicted = self
            .playback
            .apply(&event, voicemod_log::instant_of(line.at, now), sound);
        let started = matches!(
            event,
            LogEvent::Played {
                from_start: true,
                state: State::Playing,
                ..
            }
        );
        unpredicted && started
    }

    /// Takes what plays from the log, when followed.
    fn resync(&mut self, now: Instant) {
        let Some(as_of) = self.log.as_of().filter(|_| self.playback.follows_log()) else {
            return;
        };
        let logged: Vec<LoggedSound> = self
            .log
            .active()
            .map(|active| LoggedSound {
                sound: self.sounds.get(active.id),
                state: active.state,
                started: voicemod_log::instant_of(active.started, now),
                id: active.id,
            })
            .collect();
        self.playback.resync(&logged, voicemod_log::instant_of(as_of, now), now);
    }

    fn page_id(&self, index: usize) -> Option<(String, usize)> {
        let page = self.pages.get(index)?;
        Some((page.soundboard_id.clone(), page.part))
    }

    fn request(&mut self, request: Request) {
        if self.voicemod == VoicemodStatus::Connected {
            self.outputs.push(Output::Voicemod(request));
        }
    }

    fn sound_at(&self, pad: u8) -> Option<&Sound> {
        self.pages.get(self.page)?.sounds.get(usize::from(pad))?.as_ref()
    }

    fn refresh_leds(&mut self, now: Instant) {
        for index in 0..LED_COUNT {
            let led = self.led(index, now);
            if self.leds[index] != Some(led) {
                self.leds[index] = Some(led);
                self.outputs.push(Output::Board(led.message(index)));
            }
        }
    }

    fn led(&self, index: usize, now: Instant) -> Led {
        let unavailable = matches!(
            self.voicemod,
            VoicemodStatus::Unreachable | VoicemodStatus::KeyRejected | VoicemodStatus::MissingKey
        );
        if unavailable {
            Led {
                color: RED,
                effect: Effect::Blink,
            }
        } else if index < PADS {
            self.pad_led(index as u8, now)
        } else {
            self.button_led((index - PADS) as u8)
        }
    }

    fn pad_led(&self, pad: u8, now: Instant) -> Led {
        if let Some(row) = self.knob.row.filter(|_| self.knob.pressed) {
            if usize::from(pad) / COLUMNS == row {
                return Led::solid(HIGHLIGHT);
            }
            let led = self.sound_led(pad);
            let dimmed = led.color.map(|level| (f32::from(level) * 0.2).round() as u8);
            return Led { color: dimmed, ..led };
        }
        if self
            .peek
            .as_ref()
            .is_some_and(|peek| peek.pad == pad && peek.until > now)
        {
            return Led {
                color: HIGHLIGHT,
                effect: Effect::Blink,
            };
        }
        if self.flashes.contains_key(&pad) {
            return Led::solid(YELLOW);
        }
        self.sound_led(pad)
    }

    fn sound_led(&self, pad: u8) -> Led {
        match self.sound_at(pad) {
            None => Led::OFF,
            Some(sound) if !sound.enabled => Led::solid(DIM_RED),
            Some(sound) => match self.playback.state(&sound.id) {
                None => Led::solid(BLUE),
                Some(State::Playing) => Led::solid(GREEN),
                Some(State::Paused) => Led {
                    color: GREEN,
                    effect: Effect::Breathe,
                },
            },
        }
    }

    fn button_led(&self, button: u8) -> Led {
        let lit = |on: bool, color| if on { Led::solid(color) } else { Led::OFF };
        match button {
            PREV | NEXT => lit(self.pages.len() > 1, WHITE),
            STOP => lit(!self.playback.is_empty(), RED),
            MUTE_ME => lit(self.switches.mute_for_me, AMBER),
            VOICE => lit(self.switches.voice_changer, GREEN),
            HEAR_ME => lit(self.switches.hear_myself, GREEN),
            _ => Led::OFF,
        }
    }
}

#[cfg(test)]
mod tests;
