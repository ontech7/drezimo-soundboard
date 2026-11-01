//! What the device does, as in the simulator: the pads play the sounds of the current page, the function keys and
//! the knob drive Voicemod and the pages, and the LEDs show what Voicemod is doing. Plain logic: it gets each input
//! with the time it happened and queues the messages for the board and the requests for Voicemod.

use crate::playback::{Playback, State};
use crate::protocol::{BoardMessage, Effect, LED_COUNT, Led, PAD_COUNT};
use crate::soundboard::{self, Page, PlaybackMode, Sound, Soundboard};
use crate::voicemod::{Request, VoicemodEvent, VoicemodStatus};
use serde::Serialize;
use std::collections::HashMap;
use std::mem;
use std::time::{Duration, Instant};

const PADS: usize = PAD_COUNT as usize;
/// How long a pressed pad shows yellow
const PRESS_FLASH: Duration = Duration::from_millis(200);
/// A key-up sent right after the key-down can leave a sound stuck in Voicemod (docs/project.md, section 8)
const MIN_HOLD: Duration = Duration::from_millis(200);

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

#[derive(Debug, PartialEq)]
pub enum Output {
    Board(String),
    Voicemod(Request),
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
struct Switches {
    mute_for_me: bool,
    voice_changer: bool,
    hear_myself: bool,
}

pub struct Controller {
    voicemod: VoicemodStatus,
    switches: Switches,
    pages: Vec<Page>,
    page: usize,
    /// The page to show again when the soundboards come back after Voicemod was away: soundboard and part
    resume: Option<(String, usize)>,
    playback: Playback,
    /// The pads held down. The sound is kept, so its key-up still reaches it after a page change.
    held: HashMap<u8, Option<HeldSound>>,
    /// Until when each pressed pad shows yellow
    flashes: HashMap<u8, Instant>,
    /// Key-ups waiting for the minimum hold, and when they are due
    releases: Vec<(Instant, HeldSound)>,
    board_connected: bool,
    /// What the board's LEDs show, when known
    leds: [Option<Led>; LED_COUNT],
    outputs: Vec<Output>,
}

impl Default for Controller {
    fn default() -> Self {
        Self {
            voicemod: VoicemodStatus::default(),
            switches: Switches::default(),
            pages: Vec::new(),
            page: 0,
            resume: None,
            playback: Playback::default(),
            held: HashMap::new(),
            flashes: HashMap::new(),
            releases: Vec::new(),
            board_connected: false,
            leds: [None; LED_COUNT],
            outputs: Vec::new(),
        }
    }
}

impl Controller {
    pub fn set_board_connected(&mut self, connected: bool, now: Instant) {
        if connected == self.board_connected {
            return;
        }
        self.board_connected = connected;
        // A board that connects waits for every LED; one that went away cannot report its keys' release
        self.leds = [None; LED_COUNT];
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
            BoardMessage::Button { index, pressed: true } => self.button_pressed(index),
            BoardMessage::Encoder { step } => self.turn_page(step.into()),
            _ => {}
        }
    }

    pub fn voicemod_event(&mut self, event: VoicemodEvent) {
        match event {
            VoicemodEvent::Status(status) => {
                self.voicemod = status;
                // What played before is unknown now, the waiting key-ups can no longer reach it, and the soundboards
                // may change meanwhile: Voicemod sends them again once connected
                self.playback.stop_all();
                self.releases.clear();
                self.switches = Switches::default();
                if let Some(page) = self.page_id() {
                    self.resume = Some(page);
                }
                self.pages.clear();
                self.page = 0;
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

    /// Applies what is due by `now` and queues the LED changes.
    pub fn update(&mut self, now: Instant) {
        let (due, waiting): (Vec<_>, Vec<_>) = mem::take(&mut self.releases)
            .into_iter()
            .partition(|(at, _)| *at <= now);
        self.releases = waiting;
        for (_, HeldSound { sound, .. }) in due {
            // Not to a sound stopped meanwhile: Voicemod would take the key-up as a press and start it again
            if self.playback.state(&sound.id).is_some() {
                self.playback.release(&sound);
                self.request(Request::PlaySound {
                    id: sound.id,
                    key_down: false,
                });
            }
        }
        self.flashes.retain(|_, until| *until > now);
        self.playback.update(now);
        if self.board_connected {
            self.refresh_leds();
        }
    }

    /// When `update` has something to do without any input.
    pub fn next_update(&self) -> Option<Instant> {
        let flashes = self.flashes.values().copied();
        let releases = self.releases.iter().map(|(at, _)| *at);
        flashes.chain(releases).chain(self.playback.next_end()).min()
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

    fn button_pressed(&mut self, button: u8) {
        match button {
            PREV => self.turn_page(-1),
            NEXT => self.turn_page(1),
            STOP => {
                self.playback.stop_all();
                self.request(Request::StopAllSounds);
            }
            MUTE_ME => self.request(Request::ToggleMuteForMe),
            VOICE => self.request(Request::ToggleVoiceChanger),
            HEAR_ME => self.request(Request::ToggleHearMyself),
            _ => {}
        }
    }

    fn turn_page(&mut self, step: isize) {
        let count = self.pages.len() as isize;
        if count > 0 {
            self.page = (self.page as isize + step).rem_euclid(count) as usize;
        }
    }

    fn set_soundboards(&mut self, soundboards: &[Soundboard]) {
        let wanted = self.resume.take().or_else(|| self.page_id());
        self.pages = soundboard::pages(soundboards);
        self.page = wanted
            .and_then(|(id, part)| {
                let same_soundboard = |page: &Page| page.soundboard_id == id;
                let exact = self
                    .pages
                    .iter()
                    .position(|page| same_soundboard(page) && page.part == part);
                // A soundboard that shrank keeps its last page
                exact.or_else(|| self.pages.iter().rposition(same_soundboard))
            })
            .unwrap_or(0);
    }

    fn page_id(&self) -> Option<(String, usize)> {
        let page = self.pages.get(self.page)?;
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

    fn refresh_leds(&mut self) {
        for index in 0..LED_COUNT {
            let led = self.led(index);
            if self.leds[index] != Some(led) {
                self.leds[index] = Some(led);
                self.outputs.push(Output::Board(led.message(index)));
            }
        }
    }

    fn led(&self, index: usize) -> Led {
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
            self.pad_led(index as u8)
        } else {
            self.button_led((index - PADS) as u8)
        }
    }

    fn pad_led(&self, pad: u8) -> Led {
        if self.flashes.contains_key(&pad) {
            return Led::solid(YELLOW);
        }
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
mod tests {
    use super::*;
    use crate::soundboard::tests::{sound, soundboard};

    const STOP_LED: usize = PADS + STOP as usize;

    /// A controller with Voicemod and the board connected, its first LEDs already sent.
    fn ready(soundboards: Vec<Soundboard>) -> (Controller, Instant) {
        let now = Instant::now();
        let mut controller = Controller::default();
        controller.voicemod_event(VoicemodEvent::Status(VoicemodStatus::Connected));
        controller.voicemod_event(VoicemodEvent::Soundboards(soundboards));
        controller.set_board_connected(true, now);
        controller.update(now);
        controller.take_outputs();
        (controller, now)
    }

    fn press(controller: &mut Controller, pad: u8, pressed: bool, now: Instant) -> Vec<Output> {
        controller.board_message(BoardMessage::Pad { index: pad, pressed }, now);
        controller.update(now);
        controller.take_outputs()
    }

    fn button(controller: &mut Controller, index: u8, now: Instant) -> Vec<Output> {
        controller.board_message(BoardMessage::Button { index, pressed: true }, now);
        controller.update(now);
        controller.take_outputs()
    }

    fn requests(outputs: &[Output]) -> Vec<&Request> {
        outputs
            .iter()
            .filter_map(|output| match output {
                Output::Voicemod(request) => Some(request),
                Output::Board(_) => None,
            })
            .collect()
    }

    fn led(outputs: &[Output], index: usize) -> Option<&str> {
        let prefix = format!("LED,{index},");
        outputs.iter().rev().find_map(|output| match output {
            Output::Board(line) if line.starts_with(&prefix) => Some(&line[prefix.len()..]),
            _ => None,
        })
    }

    fn play(id: &str, key_down: bool) -> Request {
        Request::PlaySound {
            id: id.into(),
            key_down,
        }
    }

    #[test]
    fn sends_every_led_when_the_board_connects() {
        let locked = Sound {
            enabled: false,
            ..sound("locked")
        };
        let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a"), locked])]);
        controller.set_board_connected(false, now);
        controller.set_board_connected(true, now);
        controller.update(now);
        let outputs = controller.take_outputs();

        assert_eq!(outputs.len(), LED_COUNT);
        assert_eq!(led(&outputs, 0), Some("47,107,255"));
        assert_eq!(led(&outputs, 1), Some("90,16,16"));
        assert_eq!(led(&outputs, 2), Some("0,0,0"));
        assert_eq!(led(&outputs, STOP_LED), Some("0,0,0"));
    }

    #[test]
    fn a_pad_plays_its_sound_with_a_single_press() {
        let (mut controller, start) = ready(vec![soundboard("b", vec![sound("a")])]);

        let outputs = press(&mut controller, 0, true, start);
        assert_eq!(requests(&outputs), [&play("a", true)]);
        assert_eq!(led(&outputs, 0), Some("255,210,26"));
        assert_eq!(led(&outputs, STOP_LED), Some("255,42,42"));

        assert!(press(&mut controller, 0, false, start).is_empty());

        let flash_end = start + PRESS_FLASH;
        assert_eq!(controller.next_update(), Some(flash_end));
        controller.update(flash_end);
        assert_eq!(led(&controller.take_outputs(), 0), Some("34,224,90"));

        let end = controller.next_update().unwrap();
        controller.update(end);
        let outputs = controller.take_outputs();
        assert_eq!(led(&outputs, 0), Some("47,107,255"));
        assert_eq!(led(&outputs, STOP_LED), Some("0,0,0"));
        assert_eq!(controller.next_update(), None);
    }

    #[test]
    fn a_held_sound_gets_its_key_up_after_the_minimum_hold() {
        let held = Sound {
            playback_mode: PlaybackMode::PlayLoopOnPress,
            ..sound("held")
        };
        let (mut controller, start) = ready(vec![soundboard("b", vec![held])]);
        press(&mut controller, 0, true, start);

        let early = start + Duration::from_millis(50);
        assert!(requests(&press(&mut controller, 0, false, early)).is_empty());
        assert_eq!(controller.next_update(), Some(start + MIN_HOLD));

        controller.update(start + MIN_HOLD);
        assert_eq!(requests(&controller.take_outputs()), [&play("held", false)]);
    }

    #[test]
    fn pressing_a_held_sound_again_before_its_key_up_keeps_it_playing() {
        let held = Sound {
            playback_mode: PlaybackMode::PlayLoopOnPress,
            ..sound("held")
        };
        let (mut controller, start) = ready(vec![soundboard("b", vec![held])]);
        press(&mut controller, 0, true, start);
        press(&mut controller, 0, false, start + Duration::from_millis(50));

        let again = start + Duration::from_millis(100);
        assert!(requests(&press(&mut controller, 0, true, again)).is_empty());
        controller.update(start + MIN_HOLD);
        assert!(requests(&controller.take_outputs()).is_empty());

        let released = start + Duration::from_secs(1);
        assert_eq!(
            requests(&press(&mut controller, 0, false, released)),
            [&play("held", false)]
        );
    }

    #[test]
    fn a_held_sound_stopped_before_its_key_up_plays_again_when_pressed() {
        let held = Sound {
            playback_mode: PlaybackMode::PlayLoopOnPress,
            ..sound("held")
        };
        let (mut controller, start) = ready(vec![soundboard("b", vec![held])]);
        press(&mut controller, 0, true, start);
        press(&mut controller, 0, false, start + Duration::from_millis(50));
        button(&mut controller, STOP, start + Duration::from_millis(80));

        let again = start + Duration::from_millis(100);
        assert_eq!(requests(&press(&mut controller, 0, true, again)), [&play("held", true)]);
        controller.update(start + MIN_HOLD);
        assert!(requests(&controller.take_outputs()).is_empty());
    }

    #[test]
    fn a_stopped_held_sound_gets_no_key_up() {
        let held = Sound {
            playback_mode: PlaybackMode::PlayLoopOnPress,
            ..sound("held")
        };
        let (mut controller, start) = ready(vec![soundboard("b", vec![held])]);
        press(&mut controller, 0, true, start);
        button(&mut controller, STOP, start);

        let released = start + Duration::from_secs(1);
        assert!(requests(&press(&mut controller, 0, false, released)).is_empty());
        assert_eq!(controller.next_update(), None);
    }

    #[test]
    fn locked_and_empty_pads_play_nothing() {
        let locked = Sound {
            enabled: false,
            ..sound("locked")
        };
        let (mut controller, now) = ready(vec![soundboard("b", vec![locked])]);
        for pad in [0, 1] {
            let outputs = press(&mut controller, pad, true, now);
            assert!(requests(&outputs).is_empty());
            assert_eq!(led(&outputs, pad.into()), Some("255,210,26"));
        }
    }

    #[test]
    fn function_keys_drive_voicemod() {
        let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a")])]);
        press(&mut controller, 0, true, now);

        let outputs = button(&mut controller, STOP, now);
        assert_eq!(requests(&outputs), [&Request::StopAllSounds]);
        assert_eq!(led(&outputs, STOP_LED), Some("0,0,0"));

        let outputs = button(&mut controller, MUTE_ME, now);
        assert_eq!(requests(&outputs), [&Request::ToggleMuteForMe]);
        controller.voicemod_event(VoicemodEvent::MuteForMe(true));
        controller.update(now);
        assert_eq!(
            led(&controller.take_outputs(), PADS + MUTE_ME as usize),
            Some("255,140,0")
        );

        assert_eq!(
            requests(&button(&mut controller, VOICE, now)),
            [&Request::ToggleVoiceChanger]
        );
        assert_eq!(
            requests(&button(&mut controller, HEAR_ME, now)),
            [&Request::ToggleHearMyself]
        );
    }

    #[test]
    fn pages_turn_around_and_follow_voicemod() {
        let (mut controller, now) = ready(vec![
            soundboard("a", vec![sound("a1")]),
            soundboard("b", vec![sound("b1")]),
        ]);
        assert_eq!(controller.page().unwrap().number, 1);

        controller.board_message(BoardMessage::Encoder { step: -1 }, now);
        assert_eq!(controller.page().unwrap().name, "B");
        button(&mut controller, NEXT, now);
        assert_eq!(controller.page().unwrap().name, "A");

        controller.voicemod_event(VoicemodEvent::ActiveSoundboard("b".into()));
        assert_eq!(controller.page().unwrap().number, 2);
        assert_eq!(requests(&press(&mut controller, 0, true, now)), [&play("b1", true)]);
    }

    #[test]
    fn keeps_the_page_when_the_soundboards_change() {
        let (mut controller, _) = ready(vec![soundboard("a", vec![]), soundboard("b", vec![])]);
        controller.voicemod_event(VoicemodEvent::ActiveSoundboard("b".into()));

        controller.voicemod_event(VoicemodEvent::Soundboards(vec![
            soundboard("new", vec![]),
            soundboard("a", vec![]),
            soundboard("b", vec![]),
        ]));
        assert_eq!(controller.page().unwrap().number, 3);

        controller.voicemod_event(VoicemodEvent::Soundboards(vec![soundboard("a", vec![])]));
        assert_eq!(controller.page().unwrap().number, 1);
    }

    #[test]
    fn a_soundboard_that_shrank_keeps_its_last_page() {
        let sounds = |count: usize| (0..count).map(|i| sound(&format!("s{i}"))).collect::<Vec<_>>();
        let (mut controller, now) = ready(vec![soundboard("a", vec![]), soundboard("big", sounds(80))]);
        controller.board_message(BoardMessage::Encoder { step: -1 }, now);
        assert_eq!(controller.page().unwrap().part, 3);

        controller.voicemod_event(VoicemodEvent::Soundboards(vec![
            soundboard("a", vec![]),
            soundboard("big", sounds(40)),
        ]));
        let page = controller.page().unwrap();
        assert_eq!((page.name.as_str(), page.part), ("BIG", 2));
    }

    #[test]
    fn returns_to_its_page_when_voicemod_comes_back() {
        let soundboards = || vec![soundboard("a", vec![sound("a1")]), soundboard("b", vec![sound("b1")])];
        let (mut controller, now) = ready(soundboards());
        controller.voicemod_event(VoicemodEvent::ActiveSoundboard("b".into()));

        controller.voicemod_event(VoicemodEvent::Status(VoicemodStatus::Unreachable));
        assert_eq!(controller.page(), None);
        controller.voicemod_event(VoicemodEvent::Status(VoicemodStatus::Connected));
        assert!(
            requests(&press(&mut controller, 0, true, now)).is_empty(),
            "no sound before the soundboards"
        );

        controller.voicemod_event(VoicemodEvent::Soundboards(soundboards()));
        assert_eq!(controller.page().unwrap().name, "B");
    }

    #[test]
    fn blinks_red_and_plays_nothing_without_voicemod() {
        let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a")])]);
        controller.voicemod_event(VoicemodEvent::Status(VoicemodStatus::Unreachable));
        controller.update(now);
        let outputs = controller.take_outputs();
        assert_eq!(outputs.len(), LED_COUNT);
        assert!((0..LED_COUNT).all(|index| led(&outputs, index) == Some("255,42,42,BLINK")));

        assert!(requests(&press(&mut controller, 0, true, now)).is_empty());
        assert!(requests(&button(&mut controller, STOP, now)).is_empty());
    }

    #[test]
    fn releases_held_sounds_when_the_board_goes_away() {
        let held = Sound {
            playback_mode: PlaybackMode::PlayLoopOnPress,
            ..sound("held")
        };
        let (mut controller, start) = ready(vec![soundboard("b", vec![held])]);
        press(&mut controller, 0, true, start);

        let unplugged = start + Duration::from_secs(1);
        controller.set_board_connected(false, unplugged);
        controller.update(unplugged);
        assert_eq!(requests(&controller.take_outputs()), [&play("held", false)]);
    }
}
