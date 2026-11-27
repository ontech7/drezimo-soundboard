use super::overlay::OVERLAY_TIME;
use super::*;
use crate::custom_pages::{Content, CustomPad};
use crate::display::screens::{self, FRAME, Progress, Screen, SoundState};
use crate::knob::{self, Gesture, KnobSettings, PressAction, TurnAction};
use crate::macros::{Again, Clip, Level, Macro, Step, Switch, SwitchState, TimedStep, Timeline};
use crate::mixer::{Channel, gain};
use crate::playback::GUESSED_LENGTH;
use crate::press_again::PressAgain;
use crate::soundboard::PlaybackMode;
use crate::soundboard::tests::{action, custom_pad, sound, soundboard, voice};
use crate::voicemod_log::LineKind;
use std::time::SystemTime;

const STOP_LED: usize = PADS + STOP as usize;

/// A controller with Voicemod and the board connected, its first LEDs already sent.
fn ready(soundboards: Vec<Soundboard>) -> (Controller, Instant) {
    let now = Instant::now();
    let mut controller = Controller::default();
    controller.voicemod_event(VoicemodEvent::Status(VoicemodStatus::Connected), now);
    controller.voicemod_event(VoicemodEvent::Soundboards(soundboards), now);
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
            _ => None,
        })
        .collect()
}

fn led_count(outputs: &[Output]) -> usize {
    outputs
        .iter()
        .filter(|output| matches!(output, Output::Board(line) if line.starts_with("LED,")))
        .count()
}

fn led(outputs: &[Output], index: usize) -> Option<&str> {
    let prefix = format!("LED,{index},");
    outputs.iter().rev().find_map(|output| match output {
        Output::Board(line) if line.starts_with(&prefix) => Some(&line[prefix.len()..]),
        _ => None,
    })
}

fn played_in_log(id: &str, at: SystemTime) -> LogLine {
    played_file_in_log(id, at, None)
}

/// A play whose file tells its length, when readable.
fn played_file_in_log(id: &str, at: SystemTime, length: Option<Duration>) -> LogLine {
    let kind = LineKind::Play {
        id: id.into(),
        mode: PlaybackMode::PlayRestart,
        repeat: false,
        paused: false,
        length,
    };
    LogLine { at, kind }
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

    assert_eq!(led_count(&outputs), LED_COUNT);
    assert!(
        outputs.iter().any(|output| matches!(output, Output::Display(_))),
        "and a picture"
    );
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

    assert!(requests(&press(&mut controller, 0, false, start)).is_empty());

    controller.update(start + PRESS_FLASH);
    assert_eq!(led(&controller.take_outputs(), 0), Some("34,224,90"));

    controller.update(start + Duration::from_secs(3));
    let outputs = controller.take_outputs();
    assert_eq!(led(&outputs, 0), Some("47,107,255"));
    assert_eq!(led(&outputs, STOP_LED), Some("0,0,0"));
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
    controller.update(released + MIN_HOLD);
    assert!(requests(&controller.take_outputs()).is_empty());
}

#[test]
fn locked_and_empty_pads_play_nothing() {
    let locked = Sound {
        enabled: false,
        ..sound("locked")
    };
    let (mut controller, now) = ready(vec![soundboard("b", vec![locked])]);
    for pad in [0, 1] {
        assert!(requests(&press(&mut controller, pad, true, now)).is_empty());
    }
    assert_eq!(
        led(&press(&mut controller, 2, true, now), 2),
        Some("255,210,26"),
        "an empty pad only flashes"
    );
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
    controller.voicemod_event(VoicemodEvent::MuteForMe(true), now);
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

    controller.voicemod_event(VoicemodEvent::ActiveSoundboard("b".into()), now);
    assert_eq!(controller.page().unwrap().number, 2);
    assert_eq!(requests(&press(&mut controller, 0, true, now)), [&play("b1", true)]);
}

/// Voicemod connected, its voices sent before its soundboards.
fn connected_with_voices(controller: &mut Controller, soundboards: Vec<Soundboard>, now: Instant) {
    controller.voicemod_event(VoicemodEvent::Status(VoicemodStatus::Connected), now);
    voices(controller, "dark", now);
    controller.voicemod_event(VoicemodEvent::Soundboards(soundboards), now);
}

#[test]
fn starts_on_a_page_of_sounds_rather_than_the_voices() {
    let now = Instant::now();
    let mut controller = Controller::default();
    connected_with_voices(&mut controller, vec![soundboard("a", vec![sound("a1")])], now);
    assert_eq!(controller.page().unwrap().name, "A");
}

#[test]
fn starts_on_the_page_shown_last_time() {
    let now = Instant::now();
    let mut controller = Controller::default();
    controller.start_on(("b".into(), 1));
    connected_with_voices(
        &mut controller,
        vec![soundboard("a", vec![]), soundboard("b", vec![])],
        now,
    );
    assert_eq!(controller.page().unwrap().name, "B");
}

#[test]
fn tells_the_page_to_remember_once_the_pages_are_complete() {
    let now = Instant::now();
    let mut controller = Controller::default();
    controller.voicemod_event(VoicemodEvent::Status(VoicemodStatus::Connected), now);
    voices(&mut controller, "dark", now);
    assert_eq!(controller.shown_page(), None);

    controller.voicemod_event(
        VoicemodEvent::Soundboards(vec![soundboard("a", vec![]), soundboard("b", vec![])]),
        now,
    );
    button(&mut controller, NEXT, now);
    assert_eq!(controller.shown_page(), Some(("b".into(), 1)));

    controller.voicemod_event(VoicemodEvent::Status(VoicemodStatus::Unreachable), now);
    assert_eq!(controller.shown_page(), None);
}

#[test]
fn keeps_the_page_when_the_soundboards_change() {
    let (mut controller, now) = ready(vec![soundboard("a", vec![]), soundboard("b", vec![])]);
    controller.voicemod_event(VoicemodEvent::ActiveSoundboard("b".into()), now);

    controller.voicemod_event(
        VoicemodEvent::Soundboards(vec![
            soundboard("new", vec![]),
            soundboard("a", vec![]),
            soundboard("b", vec![]),
        ]),
        now,
    );
    assert_eq!(controller.page().unwrap().number, 3);

    controller.voicemod_event(VoicemodEvent::Soundboards(vec![soundboard("a", vec![])]), now);
    assert_eq!(controller.page().unwrap().number, 1);
}

#[test]
fn a_soundboard_that_shrank_keeps_its_last_page() {
    let sounds = |count: usize| (0..count).map(|i| sound(&format!("s{i}"))).collect::<Vec<_>>();
    let (mut controller, now) = ready(vec![soundboard("a", vec![]), soundboard("big", sounds(80))]);
    controller.board_message(BoardMessage::Encoder { step: -1 }, now);
    assert_eq!(controller.page().unwrap().part, 3);

    controller.voicemod_event(
        VoicemodEvent::Soundboards(vec![soundboard("a", vec![]), soundboard("big", sounds(40))]),
        now,
    );
    let page = controller.page().unwrap();
    assert_eq!((page.name.as_str(), page.part), ("BIG", 2));
}

#[test]
fn returns_to_its_page_when_voicemod_comes_back() {
    let soundboards = || vec![soundboard("a", vec![sound("a1")]), soundboard("b", vec![sound("b1")])];
    let (mut controller, now) = ready(soundboards());
    controller.voicemod_event(VoicemodEvent::ActiveSoundboard("b".into()), now);

    controller.voicemod_event(VoicemodEvent::Status(VoicemodStatus::Unreachable), now);
    assert_eq!(controller.page(), None);
    controller.voicemod_event(VoicemodEvent::Status(VoicemodStatus::Connected), now);
    assert!(
        requests(&press(&mut controller, 0, true, now)).is_empty(),
        "no sound before the soundboards"
    );

    controller.voicemod_event(VoicemodEvent::Soundboards(soundboards()), now);
    assert_eq!(controller.page().unwrap().name, "B");
}

#[test]
fn blinks_red_and_plays_nothing_without_voicemod() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a")])]);
    controller.voicemod_event(VoicemodEvent::Status(VoicemodStatus::Unreachable), now);
    controller.update(now);
    let outputs = controller.take_outputs();
    assert_eq!(led_count(&outputs), LED_COUNT);
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

#[test]
fn learns_the_lengths_from_the_log() {
    let (mut controller, start) = ready(vec![soundboard("b", vec![sound("a")])]);
    let earlier = SystemTime::now() - Duration::from_secs(60);
    let ended = LogLine {
        at: earlier + Duration::from_millis(850),
        kind: LineKind::End { id: "a".into() },
    };
    controller.log_input(LogInput::Opened(vec![played_in_log("a", earlier), ended]), start);

    press(&mut controller, 0, true, start);
    controller.update(start + Duration::from_millis(850));
    assert_eq!(led(&controller.take_outputs(), 0), Some("47,107,255"));
}

#[test]
fn shows_sounds_played_from_voicemod_itself() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a")])]);
    controller.log_input(LogInput::Opened(Vec::new()), now);
    controller.log_input(LogInput::Lines(vec![played_in_log("a", SystemTime::now())]), now);
    controller.update(now);
    let outputs = controller.take_outputs();
    assert_eq!(led(&outputs, 0), Some("34,224,90"));
    assert_eq!(led(&outputs, STOP_LED), Some("255,42,42"));
}

#[test]
fn shows_what_already_plays_when_the_log_opens() {
    let looped = Sound {
        looped: true,
        ..sound("loop")
    };
    let (mut controller, now) = ready(vec![soundboard("b", vec![looped])]);
    let started = SystemTime::now() - Duration::from_secs(30);
    controller.log_input(LogInput::Opened(vec![played_in_log("loop", started)]), now);
    controller.update(now);
    assert_eq!(led(&controller.take_outputs(), 0), Some("34,224,90"));
}

fn knob(controller: &mut Controller, pressed: bool, now: Instant) -> Vec<Output> {
    controller.board_message(BoardMessage::EncoderButton { pressed }, now);
    controller.update(now);
    controller.take_outputs()
}

fn turn(controller: &mut Controller, step: i8, now: Instant) -> Vec<Output> {
    controller.board_message(BoardMessage::Encoder { step }, now);
    controller.update(now);
    controller.take_outputs()
}

fn three_soundboards() -> Vec<Soundboard> {
    ["a", "b", "c"]
        .into_iter()
        .map(|id| soundboard(id, vec![sound(&format!("{id}1"))]))
        .collect()
}

#[test]
fn a_tap_opens_the_menu_and_another_picks_the_page() {
    let (mut controller, now) = ready(three_soundboards());
    knob(&mut controller, true, now);
    knob(&mut controller, false, now);
    let Screen::Menu { items, selected, .. } = controller.screen(now).0 else {
        panic!("the menu");
    };
    assert_eq!((items, selected), (vec!["A".to_string(), "B".into(), "C".into()], 0));

    turn(&mut controller, 1, now);
    turn(&mut controller, 1, now);
    knob(&mut controller, true, now);
    knob(&mut controller, false, now);
    assert_eq!(controller.page().unwrap().name, "C");
    assert!(matches!(controller.screen(now).0, Screen::Home { .. }));
}

#[test]
fn the_menu_goes_round_past_its_ends() {
    let (mut controller, now) = ready(three_soundboards());
    knob(&mut controller, true, now);
    knob(&mut controller, false, now);
    turn(&mut controller, -1, now);
    let Screen::Menu { selected, .. } = controller.screen(now).0 else {
        panic!("the menu");
    };
    assert_eq!(selected, 2, "before the first, the last");
    turn(&mut controller, 1, now);
    knob(&mut controller, true, now);
    knob(&mut controller, false, now);
    assert_eq!(controller.page().unwrap().name, "A", "past the last, the first");
    assert!(matches!(controller.screen(now).0, Screen::Home { .. }));
}

#[test]
fn the_menu_closes_by_itself() {
    let (mut controller, now) = ready_without_overlay(three_soundboards());
    knob(&mut controller, true, now);
    knob(&mut controller, false, now);
    assert_eq!(controller.next_update(), Some(now + MENU_TIME));
    controller.update(now + MENU_TIME);
    assert!(matches!(controller.screen(now + MENU_TIME).0, Screen::Home { .. }));
    assert_eq!(controller.page().unwrap().number, 1);
}

#[test]
fn holding_the_knob_peeks_without_playing() {
    let stops_others = Sound {
        stop_other_sounds: true,
        ..sound("honk")
    };
    let (mut controller, now) = ready(vec![soundboard("b", vec![stops_others])]);
    knob(&mut controller, true, now);
    let outputs = press(&mut controller, 0, true, now);
    assert!(requests(&outputs).is_empty());
    assert_eq!(led(&outputs, 0), Some("200,200,210,BLINK"));
    let Screen::Peek {
        position, name, note, ..
    } = controller.screen(now).0
    else {
        panic!("a peek");
    };
    assert_eq!(
        (position.as_str(), name.as_str(), note.as_str()),
        ("R1 C1", "HONK", "STOPS OTHERS")
    );

    press(&mut controller, 0, false, now);
    knob(&mut controller, false, now);
    assert!(controller.menu.is_none(), "a peek is not a tap");
    controller.update(now + PEEK_TIME);
    assert!(matches!(controller.screen(now + PEEK_TIME).0, Screen::Home { .. }));
}

#[test]
fn holding_and_turning_shows_a_row() {
    let sounds = (0..12).map(|i| sound(&format!("s{i}"))).collect();
    let (mut controller, now) = ready(vec![soundboard("b", sounds)]);
    knob(&mut controller, true, now);
    let outputs = turn(&mut controller, 1, now);
    assert_eq!(led(&outputs, 0), Some("200,200,210"));
    assert_eq!(led(&outputs, 6), Some("9,21,51"), "the other rows dimmed");

    let outputs = turn(&mut controller, 1, now);
    assert_eq!(led(&outputs, 6), Some("200,200,210"));
    let Screen::Row { heading, names, .. } = controller.screen(now).0 else {
        panic!("a row");
    };
    assert_eq!(heading, "ROW 2/6");
    assert_eq!(names[0].as_deref(), Some("S6"));

    let outputs = knob(&mut controller, false, now);
    assert_eq!(led(&outputs, 6), Some("47,107,255"));
    assert_eq!(
        controller.page().unwrap().number,
        1,
        "turning while held is not a page change"
    );
}

#[test]
fn the_display_shows_the_last_sound_and_its_countdown() {
    let (mut controller, start) = ready(vec![soundboard("b", vec![sound("honk")])]);
    let outputs = press(&mut controller, 0, true, start);
    assert!(outputs.iter().any(|output| matches!(output, Output::Display(_))));
    let screen = controller.screen(start).0;
    assert_eq!((screen.name(), time_shown(&screen)), (Some("HONK"), "3.0s"));
    assert!(
        controller
            .next_update()
            .is_some_and(|at| at <= start + Duration::from_millis(100))
    );

    let later = start + Duration::from_millis(1500);
    controller.update(later);
    assert_eq!(time_shown(&controller.screen(later).0), "1.5s");
}

#[test]
fn a_sound_started_from_the_computer_wakes_the_display() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a")])]);
    controller.log_input(LogInput::Opened(Vec::new()), now);
    controller.log_input(LogInput::Lines(vec![played_in_log("a", SystemTime::now())]), now);
    let outputs = controller.take_outputs();
    assert!(outputs.contains(&Output::Board("WAKE".into())));

    let outputs = press(&mut controller, 0, true, now + Duration::from_secs(5));
    assert!(
        !outputs.contains(&Output::Board("WAKE".into())),
        "the board wakes by itself on a press"
    );
}

#[test]
fn redraws_the_display_only_when_it_changes() {
    let (mut controller, now) = ready(three_soundboards());
    controller.update(now + Duration::from_secs(1));
    let outputs = controller.take_outputs();
    assert!(!outputs.iter().any(|output| matches!(output, Output::Display(_))));
}

#[test]
fn the_open_menu_keeps_its_selection_when_the_soundboards_change() {
    let (mut controller, now) = ready(three_soundboards());
    knob(&mut controller, true, now);
    knob(&mut controller, false, now);
    turn(&mut controller, 1, now);

    let mut soundboards = three_soundboards();
    soundboards.remove(0);
    controller.voicemod_event(VoicemodEvent::Soundboards(soundboards), now);
    let Screen::Menu { items, selected, .. } = controller.screen(now).0 else {
        panic!("the menu");
    };
    assert_eq!(items[selected], "B");
}

#[test]
fn asks_for_a_new_app_when_voicemod_refuses_the_key() {
    let (mut controller, now) = ready(three_soundboards());
    controller.voicemod_event(VoicemodEvent::Status(VoicemodStatus::KeyRejected), now);
    let Screen::Voicemod { message, .. } = controller.screen(now).0 else {
        panic!("the Voicemod screen");
    };
    assert_eq!(message, ENGLISH.update_app);
}

#[test]
fn shows_no_counter_without_soundboards() {
    let (controller, now) = ready(Vec::new());
    let Screen::Home { counter, name, .. } = controller.screen(now).0 else {
        panic!("the home screen");
    };
    assert_eq!((counter.as_str(), name.as_str()), ("", ENGLISH.no_sound));
}

#[test]
fn shows_the_name_of_a_sound_started_from_the_computer() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("honk")])]);
    controller.log_input(LogInput::Opened(Vec::new()), now);
    controller.log_input(LogInput::Lines(vec![played_in_log("honk", SystemTime::now())]), now);
    controller.update(now);
    let Screen::Home { name, .. } = controller.screen(now).0 else {
        panic!("the home screen");
    };
    assert_eq!(name, "HONK");
}

#[test]
fn shows_a_community_sound_and_its_countdown() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("honk")])]);
    controller.log_input(LogInput::Opened(Vec::new()), now);
    let community = played_file_in_log("2597e302", SystemTime::now(), Some(Duration::from_millis(1800)));
    controller.log_input(LogInput::Lines(vec![community]), now);
    let screen = controller.screen(now).0;
    assert_eq!((screen.name(), time_shown(&screen)), (Some(ENGLISH.community), "1.8s"));
}

#[test]
fn a_measured_length_wins_over_the_files() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("honk")])]);
    let earlier = SystemTime::now() - Duration::from_secs(60);
    let file_length = Some(Duration::from_secs(3));
    let ended = LogLine {
        at: earlier + Duration::from_secs(2),
        kind: LineKind::End { id: "2597e302".into() },
    };
    let history = vec![played_file_in_log("2597e302", earlier, file_length), ended];
    controller.log_input(LogInput::Opened(history), now);
    let again = played_file_in_log("2597e302", SystemTime::now(), file_length);
    controller.log_input(LogInput::Lines(vec![again]), now);
    assert_eq!(time_shown(&controller.screen(now).0), "2.0s");
}

#[test]
fn shows_a_sound_with_no_known_end_and_the_switches() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("honk")])]);
    controller.voicemod_event(VoicemodEvent::MuteForMe(true), now);
    controller.log_input(LogInput::Opened(Vec::new()), now);
    controller.log_input(LogInput::Lines(vec![played_in_log("2597e302", SystemTime::now())]), now);
    let Screen::Home {
        name,
        progress,
        switches,
        ..
    } = controller.screen(now).0
    else {
        panic!("the home screen");
    };
    assert_eq!(name, ENGLISH.community);
    let Some(Progress::Sound { left, time, .. }) = progress else {
        panic!("the sound");
    };
    assert_eq!((left, time.as_str()), (None, ""));
    assert_eq!(switches, [screens::Switch::MuteForMe]);
}

#[test]
fn shows_the_voice_loaded_or_the_voice_changer_off() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("honk")])]);
    controller.voicemod_event(
        VoicemodEvent::Voices {
            voices: vec![voice("dark")],
            current: Some("dark".into()),
        },
        now,
    );
    controller.voicemod_event(VoicemodEvent::VoiceChanger(true), now);
    let voice_shown = |controller: &Controller| match controller.screen(now).0 {
        Screen::Home { voice, voice_on, .. } => (voice, voice_on),
        _ => panic!("the home screen"),
    };
    assert_eq!(voice_shown(&controller), (Some("DARK".to_string()), true));
    controller.voicemod_event(VoicemodEvent::VoiceChanger(false), now);
    assert_eq!(voice_shown(&controller), (Some(ENGLISH.voice_off.to_string()), false));
}

#[test]
fn a_loops_bar_follows_its_rounds_and_waits_while_paused() {
    let looped = Sound {
        looped: true,
        playback_mode: PlaybackMode::PlayPause,
        ..sound("crickets")
    };
    let (mut controller, start) = ready(vec![soundboard("b", vec![looped])]);
    controller.playback.learn_length("crickets", Duration::from_secs(4));
    let left = |controller: &Controller, at: Instant| match controller.screen(at).0 {
        Screen::Home {
            progress: Some(Progress::Sound { state, left, .. }),
            ..
        } => (state, left.map(|left| (left * 100.0).round() as u32)),
        _ => panic!("a sound"),
    };
    let second = Duration::from_secs(1);

    press(&mut controller, 0, true, start);
    press(&mut controller, 0, false, start);
    assert_eq!(left(&controller, start + second), (SoundState::Looping, Some(75)));
    press(&mut controller, 0, true, start + second);
    press(&mut controller, 0, false, start + second);
    assert_eq!(left(&controller, start + second * 3), (SoundState::Paused, Some(75)));
    press(&mut controller, 0, true, start + second * 3);
    press(&mut controller, 0, false, start + second * 3);
    assert_eq!(
        left(&controller, start + second * 6),
        (SoundState::Looping, Some(100)),
        "a round and a quarter later"
    );
}

#[test]
fn a_sound_still_playing_keeps_its_name() {
    let (mut controller, start) = ready(vec![soundboard("b", vec![sound("honk")])]);
    controller.playback.learn_length("honk", Duration::from_secs(20));
    press(&mut controller, 0, true, start);
    let later = start + Duration::from_secs(10);
    controller.update(later);
    assert_eq!(controller.screen(later).0.name(), Some("HONK"));
}

#[test]
fn only_a_page_turn_slides_the_display() {
    let (mut controller, now) = ready(three_soundboards());
    let mut soundboards = three_soundboards();
    soundboards.insert(0, soundboard("new", vec![sound("n")]));
    controller.voicemod_event(VoicemodEvent::Soundboards(soundboards), now);
    controller.update(now);
    assert_eq!(
        controller.display.next_frame(),
        None,
        "the same page, moved down the list"
    );

    turn(&mut controller, 1, now);
    assert_eq!(controller.display.next_frame(), Some(now + FRAME));
}

/// The seconds the home screen shows below the name
fn time_shown(screen: &Screen) -> &str {
    match screen {
        Screen::Home {
            progress: Some(Progress::Sound { time, .. }),
            ..
        } => time,
        _ => "",
    }
}

/// The page the overlay shows, or None while it is hidden or shows the wheel
fn overlay_in(outputs: &[Output]) -> Option<Option<crate::overlay::Overlay>> {
    outputs.iter().rev().find_map(|output| match output {
        Output::Overlay(Some(crate::overlay::Content::Page(overlay))) => Some(Some(overlay.clone())),
        Output::Overlay(_) => Some(None),
        _ => None,
    })
}

/// A ready controller whose overlay, shown when the board connected, has gone.
fn ready_without_overlay(soundboards: Vec<Soundboard>) -> (Controller, Instant) {
    let (mut controller, start) = ready(soundboards);
    let now = start + OVERLAY_TIME;
    controller.update(now);
    assert_eq!(overlay_in(&controller.take_outputs()), Some(None));
    (controller, now)
}

#[test]
fn turning_the_page_shows_the_overlay_for_a_while() {
    let (mut controller, now) = ready_without_overlay(three_soundboards());
    let outputs = turn(&mut controller, 1, now);
    let Some(Some(shown)) = overlay_in(&outputs) else {
        panic!("the overlay");
    };
    assert_eq!((shown.title.as_str(), shown.counter.as_str()), ("B", "2/3"));
    assert_eq!(
        (shown.cells[0].name.as_str(), shown.cells[0].led),
        ("B1", Led::solid(BLUE))
    );
    assert_eq!((shown.cells[1].name.as_str(), shown.cells[1].led), ("", Led::OFF));

    // Once the new page has slid in on the display
    controller.update(now + Duration::from_millis(200));
    assert_eq!(controller.next_update(), Some(now + OVERLAY_TIME));
    controller.update(now + OVERLAY_TIME);
    assert_eq!(overlay_in(&controller.take_outputs()), Some(None));
}

#[test]
fn the_overlay_can_show_always_and_the_knob_hides_it() {
    let (mut controller, now) = ready_without_overlay(three_soundboards());
    with_knob(
        &mut controller,
        KnobSettings {
            tap: PressAction::Overlay,
            ..KnobSettings::default()
        },
    );
    controller.set_overlay_always(true);
    let later = now + OVERLAY_TIME * 3;
    assert!(controller.overlay(later).is_some(), "shown with no input");

    tap(&mut controller, later);
    let hidden = later + Duration::from_millis(100);
    controller.update(hidden);
    assert!(controller.overlay(hidden).is_none(), "hidden by the tap");
    let turned = hidden + Duration::from_millis(100);
    turn(&mut controller, 1, turned);
    assert!(
        controller.overlay(turned).is_some(),
        "a page change shows it for a while"
    );
    assert!(controller.overlay(turned + OVERLAY_TIME).is_none());

    let again = turned + OVERLAY_TIME * 2;
    tap(&mut controller, again);
    controller.update(again + Duration::from_millis(100));
    assert!(
        controller.overlay(again + OVERLAY_TIME * 5).is_some(),
        "shown again, for good"
    );
    controller.set_overlay_always(false);
    assert!(controller.overlay(again + OVERLAY_TIME * 5).is_none());
}

#[test]
fn the_knobs_overlay_gesture_shows_it_for_its_time() {
    let (mut controller, now) = ready_without_overlay(three_soundboards());
    with_knob(
        &mut controller,
        KnobSettings {
            long_press: PressAction::Overlay,
            ..KnobSettings::default()
        },
    );
    knob(&mut controller, true, now);
    let let_go = now + Duration::from_millis(700);
    knob(&mut controller, false, let_go);
    controller.update(let_go + Duration::from_millis(10));
    controller.update(let_go + OVERLAY_TIME - Duration::from_millis(10));
    assert!(
        controller
            .overlay(let_go + OVERLAY_TIME - Duration::from_millis(10))
            .is_some()
    );
    controller.update(let_go + OVERLAY_TIME);
    assert!(controller.overlay(let_go + OVERLAY_TIME).is_none());
}

#[test]
fn the_overlay_shows_the_leds() {
    let locked = Sound {
        enabled: false,
        ..sound("locked")
    };
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("honk"), locked])]);
    let outputs = press(&mut controller, 0, true, now);
    let Some(Some(shown)) = overlay_in(&outputs) else {
        panic!("the overlay");
    };
    assert_eq!(shown.cells[0].led, Led::solid(YELLOW));
    assert!(shown.cells[1].locked && !shown.cells[0].locked);
}

#[test]
fn holding_the_knob_shows_the_overlay_until_a_while_after() {
    let (mut controller, now) = ready_without_overlay(three_soundboards());
    assert!(matches!(overlay_in(&knob(&mut controller, true, now)), Some(Some(_))));
    let later = now + Duration::from_secs(10);
    controller.update(later);
    assert_ne!(overlay_in(&controller.take_outputs()), Some(None), "still shown");

    press(&mut controller, 0, true, later);
    press(&mut controller, 0, false, later);
    knob(&mut controller, false, later);
    controller.update(later + OVERLAY_TIME - Duration::from_millis(1));
    assert_ne!(overlay_in(&controller.take_outputs()), Some(None), "still shown");
    controller.update(later + OVERLAY_TIME);
    assert_eq!(overlay_in(&controller.take_outputs()), Some(None));
}

#[test]
fn no_overlay_without_the_board() {
    let (mut controller, now) = ready(three_soundboards());
    controller.set_board_connected(false, now);
    controller.update(now);
    assert_eq!(overlay_in(&controller.take_outputs()), Some(None));
    controller.voicemod_event(VoicemodEvent::ActiveSoundboard("b".into()), now);
    controller.update(now);
    assert_eq!(overlay_in(&controller.take_outputs()), None, "still hidden");
}

#[test]
fn voicemod_telling_again_of_the_soundboard_shown_is_no_page_change() {
    let (mut controller, now) = ready_without_overlay(three_soundboards());
    controller.voicemod_event(VoicemodEvent::ActiveSoundboard("a".into()), now);
    controller.update(now);
    assert_eq!(overlay_in(&controller.take_outputs()), None);

    controller.voicemod_event(VoicemodEvent::ActiveSoundboard("b".into()), now);
    controller.update(now);
    assert!(matches!(overlay_in(&controller.take_outputs()), Some(Some(_))));
    assert_eq!(controller.page().unwrap().name, "B");
}

#[test]
fn the_window_shows_the_page_and_plays_it() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("honk")])]);
    let view = controller.board_view(now).expect("a page to show");
    assert_eq!(
        (view.pads[0].name.as_str(), view.pads[0].led),
        ("HONK", Led::solid(BLUE))
    );
    assert_eq!(view.pads[1].name, "");
    assert_eq!(view.keys[usize::from(STOP)], Led::OFF);

    controller.window_pad(0, true, now);
    controller.update(now);
    assert_eq!(requests(&controller.take_outputs()), [&play("honk", true)]);
    let view = controller.board_view(now).unwrap();
    assert_eq!(view.pads[0].led, Led::solid(YELLOW));
    assert_eq!(view.keys[usize::from(STOP)], Led::solid(RED));
}

#[test]
fn a_page_picked_in_the_window_shows_no_overlay() {
    let (mut controller, now) = ready_without_overlay(three_soundboards());
    controller.go_to_page(2);
    controller.update(now);
    let view = controller.board_view(now).unwrap();
    assert_eq!(view.page, 2);
    assert_eq!(view.ids, ["a", "b", "c"], "for the window to find a page by its id");
    assert_eq!(overlay_in(&controller.take_outputs()), None);

    controller.go_to_page(3);
    assert_eq!(controller.board_view(now).unwrap().page, 2, "no such page");
}

#[test]
fn nothing_to_play_from_the_window_without_voicemod() {
    let (mut controller, now) = ready(three_soundboards());
    controller.voicemod_event(VoicemodEvent::Status(VoicemodStatus::Unreachable), now);
    assert_eq!(controller.board_view(now), None);
}

#[test]
fn the_board_and_the_window_release_only_their_own_presses() {
    let held = Sound {
        playback_mode: PlaybackMode::PlayLoopOnPress,
        ..sound("held")
    };
    let (mut controller, start) = ready(vec![soundboard("b", vec![held])]);
    controller.window_pad(0, true, start);
    controller.update(start);
    assert_eq!(requests(&controller.take_outputs()), [&play("held", true)]);
    let later = start + MIN_HOLD;
    let outputs = press(&mut controller, 0, true, later);
    assert!(requests(&outputs).is_empty(), "the window holds it");
    let outputs = press(&mut controller, 0, false, later);
    assert!(requests(&outputs).is_empty(), "only the window releases it");

    controller.set_board_connected(false, later);
    controller.update(later);
    assert!(
        requests(&controller.take_outputs()).is_empty(),
        "nor does unplugging the board"
    );

    controller.window_pad(0, false, later);
    controller.update(later);
    assert_eq!(requests(&controller.take_outputs()), [&play("held", false)]);
}

fn voices(controller: &mut Controller, current: &str, now: Instant) {
    let mut locked = voice("locked");
    locked.enabled = false;
    let event = VoicemodEvent::Voices {
        voices: vec![voice("dark"), voice("banana"), locked],
        current: Some(current.to_string()),
    };
    controller.voicemod_event(event, now);
}

#[test]
fn voices_get_a_page_and_load_from_a_pad() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a")])]);
    voices(&mut controller, "banana", now);
    assert_eq!(controller.page().unwrap().name, "B", "the page shown stays");

    controller.go_to_page(0);
    controller.update(now);
    let Screen::Home { title, .. } = controller.screen(now).0 else {
        panic!("the home screen");
    };
    assert_eq!(title, ENGLISH.voices);
    let view = controller.board_view(now).unwrap();
    let leds: Vec<Led> = view.pads[..3].iter().map(|pad| pad.led).collect();
    assert_eq!(
        leds,
        [Led::solid(PURPLE), Led::solid(GREEN), Led::OFF],
        "no locked voices"
    );
    assert!(view.pads[0].voice);

    let later = now + Duration::from_secs(1);
    let outputs = press(&mut controller, 0, true, later);
    assert_eq!(requests(&outputs), [&Request::LoadVoice("dark".into())]);
    let Screen::Home { name, .. } = controller.screen(later).0 else {
        panic!("the home screen");
    };
    assert_eq!(name, "DARK");
    assert!(requests(&press(&mut controller, 2, true, later)).is_empty());
}

#[test]
fn a_voice_picked_on_the_computer_shows_by_name() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a")])]);
    voices(&mut controller, "banana", now);
    controller.update(now);
    controller.take_outputs();

    controller.voicemod_event(VoicemodEvent::CurrentVoice("dark".into()), now);
    controller.update(now);
    assert!(controller.take_outputs().contains(&Output::Board("WAKE".into())));
    let Screen::Home { name, .. } = controller.screen(now).0 else {
        panic!("the home screen");
    };
    assert_eq!(name, "DARK");
}

#[test]
fn returns_to_its_page_whichever_list_comes_first() {
    let (mut controller, now) = ready(three_soundboards());
    controller.go_to_page(2);
    controller.voicemod_event(VoicemodEvent::Status(VoicemodStatus::Unreachable), now);
    controller.voicemod_event(VoicemodEvent::Status(VoicemodStatus::Connected), now);
    voices(&mut controller, "dark", now);
    controller.voicemod_event(VoicemodEvent::Soundboards(three_soundboards()), now);
    assert_eq!(controller.page().unwrap().name, "C");
}

#[test]
fn a_page_gone_while_voicemod_was_away_is_not_waited_for() {
    let (mut controller, now) = ready(three_soundboards());
    controller.go_to_page(2);
    controller.voicemod_event(VoicemodEvent::Status(VoicemodStatus::Unreachable), now);
    controller.voicemod_event(VoicemodEvent::Status(VoicemodStatus::Connected), now);
    let mut soundboards = three_soundboards();
    soundboards.pop();
    controller.voicemod_event(VoicemodEvent::Soundboards(soundboards.clone()), now);
    controller.go_to_page(1);
    // Voicemod sends its lists again after a sound plays
    controller.voicemod_event(VoicemodEvent::Soundboards(soundboards), now);
    assert_eq!(controller.page().unwrap().name, "B");
}

#[test]
fn a_voice_loaded_from_a_pad_shows_once() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a")])]);
    voices(&mut controller, "banana", now);
    controller.go_to_page(0);
    press(&mut controller, 0, true, now);
    let later = now + screen::LAST_SOUND_TIME;
    controller.voicemod_event(VoicemodEvent::CurrentVoice("dark".into()), later);
    controller.update(later);
    assert!(!controller.take_outputs().contains(&Output::Board("WAKE".into())));
    let Screen::Home { name, .. } = controller.screen(later).0 else {
        panic!("the home screen");
    };
    assert_eq!(name, ENGLISH.ready, "its name showed at the press");
}

/// One page of the user's own: the given pads from the top left.
fn own_page(id: &str, pads: Vec<Option<CustomPad>>) -> CustomPage {
    CustomPage {
        id: id.into(),
        name: id.to_uppercase(),
        pads,
        compact: false,
    }
}

#[test]
fn the_users_own_pages_come_first_and_follow_the_editor() {
    let (mut controller, now) = ready(three_soundboards());
    controller.go_to_page(1);
    let page = own_page("mine", vec![custom_pad(action(Action::StopSounds))]);
    controller.set_custom_pages(vec![page]);
    assert_eq!(controller.page().unwrap().name, "B", "the page shown stays");

    controller.go_to_page(0);
    assert_eq!(controller.page().unwrap().name, "MINE");
    let view = controller.board_view(now).unwrap();
    assert_eq!(
        (view.ids[0].as_str(), view.own),
        ("mine", 1),
        "the window links it to the editor"
    );
    controller.set_custom_pages(Vec::new());
    assert_eq!(
        controller.page().unwrap().name,
        "A",
        "a page deleted gives way to the first"
    );
}

#[test]
fn a_pad_with_its_own_volume_starts_its_sound_there() {
    let (mut controller, now) = ready(three_soundboards());
    bridge(&mut controller, [1.0, 1.0, 1.0], now);
    let quiet = CustomPad {
        volume: Some(30),
        ..custom_pad(Content::Sound {
            id: "a1".into(),
            name: String::new(),
        })
        .unwrap()
    };
    controller.set_custom_pages(vec![own_page("mine", vec![Some(quiet)])]);
    controller.go_to_page(0);
    let outputs = press(&mut controller, 0, true, now);
    assert_eq!(
        requests(&outputs),
        [
            &play("a1", true),
            &Request::SetSoundVolume {
                id: "a1".into(),
                volume: 0.3
            }
        ]
    );
}

#[test]
fn a_pads_own_volume_leaves_a_pause_alone() {
    let song = Sound {
        playback_mode: PlaybackMode::PlayPause,
        ..sound("song")
    };
    let (mut controller, now) = ready(vec![soundboard("b", vec![song])]);
    bridge(&mut controller, [1.0, 1.0, 1.0], now);
    let quiet = CustomPad {
        volume: Some(30),
        ..custom_pad(Content::Sound {
            id: "song".into(),
            name: String::new(),
        })
        .unwrap()
    };
    controller.set_custom_pages(vec![own_page("mine", vec![Some(quiet)])]);
    controller.go_to_page(0);
    press(&mut controller, 0, true, now);
    press(&mut controller, 0, false, now);
    let later = now + Duration::from_secs(1);
    let outputs = press(&mut controller, 0, true, later);
    assert_eq!(
        requests(&outputs),
        [&play("song", true)],
        "paused, at the volume it had"
    );
}

#[test]
fn putting_the_volumes_back_drops_a_save_on_its_way() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("song")])]);
    with_knob(
        &mut controller,
        KnobSettings {
            turn: TurnAction::PlayingSound,
            tap: PressAction::ResetVolumes,
            ..KnobSettings::default()
        },
    );
    bridge(&mut controller, [1.0, 1.0, 1.0], now);
    controller.voicemod_event(VoicemodEvent::SoundVolumes(HashMap::from([("song".into(), 0.5)])), now);
    press(&mut controller, 0, true, now);
    press(&mut controller, 0, false, now);
    turn(&mut controller, -2, now);
    tap(&mut controller, now + Duration::from_millis(200));
    controller.take_outputs();
    controller.update(now + Duration::from_secs(2));
    let saved = requests(&controller.take_outputs())
        .into_iter()
        .any(|request| matches!(request, Request::SetSoundOptions { .. }));
    assert!(!saved, "the volume put back is not saved");
}

#[test]
fn the_overlay_and_reset_pads_do_what_the_knob_does() {
    let (mut controller, now) = ready_without_overlay(three_soundboards());
    bridge(&mut controller, [1.0, 1.0, 1.0], now);
    let pads = [Action::Overlay, Action::ResetVolumes].map(|kind| custom_pad(action(kind)));
    controller.set_custom_pages(vec![own_page("mine", pads.to_vec())]);
    controller.go_to_page(0);
    controller.update(now);
    controller.take_outputs();
    press(&mut controller, 0, true, now);
    press(&mut controller, 0, false, now);
    assert!(controller.overlay(now).is_some(), "shown for its time");
    let flashed = now + Duration::from_millis(300);
    controller.update(flashed);
    assert_eq!(
        controller.board_view(flashed).unwrap().pads[0].led,
        Led::solid([255, 60, 160]),
        "lit while shown"
    );
    press(&mut controller, 1, true, now);
    assert_eq!(home_name(&controller, now), "VOLUMES RESET");
}

#[test]
fn the_knob_on_everyones_soundboard_moves_both_channels() {
    let (mut controller, now) = ready(three_soundboards());
    with_knob(
        &mut controller,
        KnobSettings {
            turn: TurnAction::SoundboardForEveryone,
            ..KnobSettings::default()
        },
    );
    bridge(&mut controller, [1.0, 0.25, 1.0], now);
    let outputs = turn(&mut controller, -2, now);
    assert_eq!(
        requests(&outputs),
        [
            &set_gain(Channel::SoundboardForOthers, gain(90)),
            &set_gain(Channel::SoundboardForMe, gain(40))
        ]
    );
    let Screen::Level { label, percent, .. } = controller.screen(now).0 else {
        panic!("the level");
    };
    assert_eq!((label.as_str(), percent), ("SOUNDBOARD ALL", 90));
}

#[test]
fn the_knob_saves_a_sounds_volume_once_it_rests() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("song")])]);
    with_knob(
        &mut controller,
        KnobSettings {
            turn: TurnAction::PlayingSound,
            ..KnobSettings::default()
        },
    );
    bridge(&mut controller, [1.0, 1.0, 1.0], now);
    controller.voicemod_event(VoicemodEvent::SoundVolumes(HashMap::from([("song".into(), 0.5)])), now);
    press(&mut controller, 0, true, now);
    press(&mut controller, 0, false, now);
    controller.take_outputs();
    turn(&mut controller, -2, now);
    let rested = now + Duration::from_secs(1);
    assert_eq!(controller.next_update().map(|at| at <= rested), Some(true));
    controller.update(rested);
    let saved = Request::SetSoundOptions {
        id: "song".into(),
        mode: None,
        volume: Some(0.4),
        stops_others: None,
    };
    assert!(requests(&controller.take_outputs()).contains(&&saved));
}

#[test]
fn action_pads_drive_voicemod() {
    let (mut controller, now) = ready(three_soundboards());
    voices(&mut controller, "dark", now);
    let pads = [
        Action::MuteMic,
        Action::RandomVoice,
        Action::Background,
        Action::StopSounds,
    ]
    .map(|kind| custom_pad(action(kind)));
    controller.set_custom_pages(vec![own_page("mine", pads.to_vec())]);
    controller.go_to_page(0);

    let pressed = |controller: &mut Controller, pad| {
        let outputs = press(controller, pad, true, now);
        press(controller, pad, false, now);
        requests(&outputs).into_iter().cloned().collect::<Vec<_>>()
    };
    assert_eq!(pressed(&mut controller, 0), [Request::ToggleMuteMic]);
    assert_eq!(pressed(&mut controller, 2), [Request::ToggleBackground]);
    assert_eq!(pressed(&mut controller, 3), [Request::StopAllSounds]);
    assert_eq!(
        pressed(&mut controller, 1),
        [Request::LoadVoice("banana".into())],
        "the only other voice in the licence"
    );
}

#[test]
fn action_pads_light_up_while_on() {
    let (mut controller, now) = ready(three_soundboards());
    let mut muted = custom_pad(action(Action::MuteMic)).unwrap();
    muted.colour = Some([10, 20, 40]);
    let pads = vec![custom_pad(action(Action::MuteMic)), Some(muted)];
    controller.set_custom_pages(vec![own_page("mine", pads)]);
    controller.go_to_page(0);
    controller.update(now);
    controller.take_outputs();
    let led_of = |controller: &Controller, pad: usize| controller.board_view(now).unwrap().pads[pad].led;
    assert_eq!(led_of(&controller, 0), Led::solid(RED.map(|level| level / 4)));

    controller.voicemod_event(VoicemodEvent::VoiceChanger(true), now);
    controller.voicemod_event(VoicemodEvent::MuteMic(true), now);
    assert_eq!(led_of(&controller, 0), Led::solid(RED));
    assert_eq!(led_of(&controller, 1), Led::solid([10, 20, 40]), "the user's colour");
    let Screen::Home { switches, .. } = controller.screen(now).0 else {
        panic!("the home screen");
    };
    assert_eq!(switches, [screens::Switch::MicOff]);
}

#[test]
fn a_pad_opens_another_page() {
    let (mut controller, now) = ready(three_soundboards());
    let go = custom_pad(Content::Action {
        action: Action::GoToPage,
        page: Some("c".into()),
    });
    controller.set_custom_pages(vec![own_page("mine", vec![go])]);
    controller.go_to_page(0);
    assert_eq!(controller.board_view(now).unwrap().pads[0].name, "Go to C");

    press(&mut controller, 0, true, now);
    assert_eq!(controller.page().unwrap().name, "C");

    let gone = custom_pad(Content::Action {
        action: Action::GoToPage,
        page: Some("gone".into()),
    });
    controller.set_custom_pages(vec![own_page("mine", vec![gone])]);
    controller.go_to_page(0);
    // Past the press's yellow flash
    let later = now + Duration::from_secs(1);
    controller.update(later);
    let pad = &controller.board_view(later).unwrap().pads[0];
    assert_eq!(
        pad.name,
        ENGLISH.action_name(Action::GoToPage),
        "a page gone is not named"
    );
    assert_eq!(pad.led, Led::solid(WHITE.map(|level| level / 4)), "nor lit");
}

#[test]
fn a_sound_takes_the_users_name_and_colour() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("honk")])]);
    let mut honk = custom_pad(Content::Sound {
        id: "honk".into(),
        name: "HONK".into(),
    })
    .unwrap();
    honk.label = "Fail".into();
    honk.colour = Some([255, 140, 40]);
    controller.set_custom_pages(vec![own_page("mine", vec![Some(honk)])]);
    controller.go_to_page(0);

    let view = controller.board_view(now).unwrap();
    assert_eq!(view.pads[0].name, "Fail");
    assert_eq!(view.pads[0].led, Led::solid([255, 140, 40]));
    let outputs = press(&mut controller, 0, true, now);
    assert_eq!(requests(&outputs), [&play("honk", true)]);
    let Screen::Home { name, .. } = controller.screen(now).0 else {
        panic!("the home screen");
    };
    assert_eq!(name, "Fail");
}

fn macro_item(id: &str, steps: Vec<Step>) -> Macro {
    Macro {
        id: id.into(),
        name: id.to_uppercase(),
        colour: None,
        animation: Default::default(),
        again: Again::Stop,
        repeat: 1,
        restore: false,
        steps,
        timeline: None,
    }
}

/// The macro on pad 0 of a page of the user's own, shown.
fn on_a_pad(controller: &mut Controller, item: Macro) {
    let pad = custom_pad(Content::Macro { id: item.id.clone() });
    controller.set_macros(vec![item]);
    controller.set_custom_pages(vec![own_page("mine", vec![pad])]);
    controller.go_to_page(0);
}

fn requests_at(controller: &mut Controller, at: Instant) -> Vec<Request> {
    controller.update(at);
    requests(&controller.take_outputs()).into_iter().cloned().collect()
}

fn clip(id: &str, track: u8, at: u32) -> Clip {
    Clip {
        id: id.into(),
        name: String::new(),
        track,
        at,
        cut: None,
        volume: Vec::new(),
    }
}

#[test]
fn a_macro_on_its_time_line_starts_each_sound_at_its_time() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a"), sound("b")])]);
    bridge(&mut controller, [1.0, 1.0, 1.0], now);
    controller.playback.learn_length("a", Duration::from_secs(4));
    controller.playback.learn_length("b", Duration::from_secs(1));
    let timeline = Timeline {
        clips: vec![
            Clip {
                cut: Some(2500),
                volume: vec![Level { at: 0, percent: 0 }, Level { at: 1000, percent: 100 }],
                ..clip("a", 0, 0)
            },
            clip("b", 1, 1000),
        ],
        actions: vec![TimedStep {
            at: 500,
            step: Step::Switch {
                switch: Switch::MuteMic,
                state: SwitchState::On,
            },
        }],
    };
    on_a_pad(
        &mut controller,
        Macro {
            timeline: Some(timeline),
            ..macro_item("macro-t", Vec::new())
        },
    );
    let at = |millis| now + Duration::from_millis(millis);
    let volume = |percent: f32| Request::SetSoundVolume {
        id: "a".into(),
        volume: percent / 100.0,
    };

    let outputs = press(&mut controller, 0, true, now);
    press(&mut controller, 0, false, now);
    assert_eq!(
        requests(&outputs),
        [&play("a", true), &volume(0.0)],
        "starts silent, by its curve"
    );
    assert_eq!(requests_at(&mut controller, at(100)), [volume(10.0)], "and rises");
    assert_eq!(
        requests_at(&mut controller, at(500)),
        [Request::ToggleMuteMic, volume(50.0)]
    );
    assert_eq!(
        requests_at(&mut controller, at(1000)),
        [play("b", true), volume(100.0)],
        "the second track's sound over the first"
    );
    assert_eq!(
        controller
            .macro_progress()
            .map(|progress| (progress.step, progress.steps)),
        Some((3, 3))
    );
    assert!(
        requests_at(&mut controller, at(1100)).is_empty(),
        "at its top, nothing more to send"
    );
    assert_eq!(
        requests_at(&mut controller, at(2500)),
        [Request::StopSound("a".into())],
        "cut"
    );
    assert_eq!(
        controller.macro_progress(),
        None,
        "over once its sounds are: b ended at 2 s, a was cut"
    );
}

#[test]
fn a_flat_volume_does_not_keep_the_controller_awake() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a")])]);
    bridge(&mut controller, [1.0, 1.0, 1.0], now);
    controller.playback.learn_length("a", Duration::from_secs(10));
    let timeline = Timeline {
        clips: vec![Clip {
            volume: vec![Level { at: 0, percent: 50 }],
            ..clip("a", 0, 0)
        }],
        actions: Vec::new(),
    };
    on_a_pad(
        &mut controller,
        Macro {
            timeline: Some(timeline),
            ..macro_item("macro-t", Vec::new())
        },
    );
    press(&mut controller, 0, true, now);
    let later = now + Duration::from_millis(300);
    controller.update(later);
    assert!(
        controller.next_update().is_none_or(|at| at > later),
        "no wake-up in the past"
    );
}

#[test]
fn a_long_sound_keeps_its_macro_going_past_30_s() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("song")])]);
    controller.voicemod_event(VoicemodEvent::SoundLengths(HashMap::from([("song".into(), 45.0)])), now);
    controller.playback.learn_length("song", Duration::from_secs(45));
    let timeline = Timeline {
        clips: vec![clip("song", 0, 0)],
        actions: Vec::new(),
    };
    on_a_pad(
        &mut controller,
        Macro {
            timeline: Some(timeline),
            ..macro_item("macro-t", Vec::new())
        },
    );
    press(&mut controller, 0, true, now);
    controller.update(now + Duration::from_secs(31));
    assert!(controller.macro_progress().is_some(), "the song still plays");
    controller.update(now + Duration::from_secs(46));
    assert_eq!(controller.macro_progress(), None);
}

#[test]
fn at_the_same_time_a_stop_comes_before_a_sound() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a")])]);
    let timeline = Timeline {
        clips: vec![clip("a", 0, 0)],
        actions: vec![TimedStep {
            at: 0,
            step: Step::StopSounds,
        }],
    };
    on_a_pad(
        &mut controller,
        Macro {
            timeline: Some(timeline),
            ..macro_item("macro-t", Vec::new())
        },
    );
    let outputs = press(&mut controller, 0, true, now);
    assert_eq!(
        requests(&outputs).last(),
        Some(&&play("a", true)),
        "the sound after the stop"
    );
}

#[test]
fn a_time_line_without_the_bridge_plays_its_sounds_whole() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a")])]);
    controller.playback.learn_length("a", Duration::from_secs(2));
    let timeline = Timeline {
        clips: vec![Clip {
            cut: Some(500),
            volume: vec![Level { at: 0, percent: 30 }],
            ..clip("a", 0, 0)
        }],
        actions: Vec::new(),
    };
    on_a_pad(
        &mut controller,
        Macro {
            timeline: Some(timeline),
            ..macro_item("macro-t", Vec::new())
        },
    );
    let outputs = press(&mut controller, 0, true, now);
    assert_eq!(requests(&outputs), [&play("a", true)], "no volume of its own");
    assert!(
        requests_at(&mut controller, now + Duration::from_millis(500)).is_empty(),
        "no cut: it would stop every sound"
    );
}

#[test]
fn a_time_line_repeats_and_restarts_from_its_start() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a")])]);
    controller.playback.learn_length("a", Duration::from_secs(1));
    let timeline = Timeline {
        clips: vec![clip("a", 0, 200)],
        actions: Vec::new(),
    };
    on_a_pad(
        &mut controller,
        Macro {
            timeline: Some(timeline),
            repeat: 2,
            again: Again::Restart,
            ..macro_item("macro-t", Vec::new())
        },
    );
    let at = |millis| now + Duration::from_millis(millis);
    press(&mut controller, 0, true, now);
    press(&mut controller, 0, false, now);
    assert_eq!(requests_at(&mut controller, at(200)), [play("a", true)]);
    controller.update(at(1300));
    assert!(controller.macro_progress().is_some(), "a second round");
    assert_eq!(
        requests_at(&mut controller, at(1500)),
        [play("a", true)],
        "200 ms into it"
    );
    controller.update(at(2600));
    assert_eq!(controller.macro_progress(), None, "two rounds");
}

#[test]
fn a_macro_runs_its_steps_in_turn_and_puts_things_back() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a")])]);
    voices(&mut controller, "banana", now);
    controller.voicemod_event(VoicemodEvent::VoiceChanger(true), now);
    controller.playback.learn_length("a", Duration::from_secs(2));
    let steps = vec![
        Step::Voice {
            id: "dark".into(),
            name: String::new(),
        },
        Step::Switch {
            switch: Switch::MuteMic,
            state: SwitchState::On,
        },
        Step::Switch {
            switch: Switch::VoiceChanger,
            state: SwitchState::On,
        },
        Step::Sound {
            id: "a".into(),
            name: String::new(),
            wait_end: true,
        },
        Step::Wait { millis: 500 },
    ];
    on_a_pad(
        &mut controller,
        Macro {
            restore: true,
            ..macro_item("macro-x", steps)
        },
    );
    let at = |millis| now + Duration::from_millis(millis);

    let outputs = press(&mut controller, 0, true, now);
    assert_eq!(
        requests(&outputs),
        [
            &Request::LoadVoice("dark".into()),
            &Request::ToggleMuteMic,
            &play("a", true)
        ],
        "the voice changer was on already"
    );
    controller.voicemod_event(VoicemodEvent::CurrentVoice("dark".into()), now);
    press(&mut controller, 0, false, now);

    assert!(requests_at(&mut controller, at(1000)).is_empty(), "the sound plays");
    let view = controller.board_view(at(1000)).unwrap();
    assert_eq!(view.pads[0].led.effect, Effect::Breathe);
    let Screen::Home {
        name,
        progress,
        switches,
        ..
    } = controller.screen(at(1000)).0
    else {
        panic!("the home screen");
    };
    assert_eq!(name, "MACRO-X");
    assert_eq!(progress, Some(Progress::Macro { step: 4, steps: 5 }));
    assert_eq!(switches, [screens::Switch::MicOff]);

    assert!(requests_at(&mut controller, at(2000)).is_empty(), "then the wait");
    assert_eq!(
        requests_at(&mut controller, at(2500)),
        [Request::LoadVoice("banana".into()), Request::ToggleMuteMic],
        "the voice and the mic as they were"
    );
    assert_eq!(controller.macro_progress(), None);
}

#[test]
fn a_macro_pressed_again_stops_or_restarts() {
    let steps = vec![
        Step::Wait { millis: 1000 },
        Step::Switch {
            switch: Switch::MuteMic,
            state: SwitchState::Toggle,
        },
    ];
    let at = |now: Instant, millis| now + Duration::from_millis(millis);

    let (mut controller, now) = ready(three_soundboards());
    on_a_pad(&mut controller, macro_item("macro-x", steps.clone()));
    press(&mut controller, 0, true, now);
    press(&mut controller, 0, false, now);
    press(&mut controller, 0, true, at(now, 500));
    assert!(requests_at(&mut controller, at(now, 1500)).is_empty(), "stopped");

    let (mut controller, now) = ready(three_soundboards());
    let restarts = Macro {
        again: Again::Restart,
        ..macro_item("macro-x", steps)
    };
    on_a_pad(&mut controller, restarts);
    press(&mut controller, 0, true, now);
    press(&mut controller, 0, false, now);
    press(&mut controller, 0, true, at(now, 500));
    assert!(requests_at(&mut controller, at(now, 1400)).is_empty(), "started again");
    assert_eq!(requests_at(&mut controller, at(now, 1500)), [Request::ToggleMuteMic]);
}

#[test]
fn a_macro_repeats_a_round_at_most_twice_a_second() {
    let (mut controller, now) = ready(three_soundboards());
    let toggle = Step::Switch {
        switch: Switch::Background,
        state: SwitchState::Toggle,
    };
    on_a_pad(
        &mut controller,
        Macro {
            repeat: 2,
            ..macro_item("macro-x", vec![toggle])
        },
    );
    let outputs = press(&mut controller, 0, true, now);
    assert_eq!(requests(&outputs), [&Request::ToggleBackground]);
    let at = |millis| now + Duration::from_millis(millis);
    assert!(requests_at(&mut controller, at(400)).is_empty());
    assert_eq!(requests_at(&mut controller, at(500)), [Request::ToggleBackground]);
    assert!(requests_at(&mut controller, at(1000)).is_empty(), "twice only");
    assert_eq!(controller.macro_progress(), None);
}

#[test]
fn the_stop_key_or_deleting_it_ends_a_macro() {
    let (mut controller, now) = ready(three_soundboards());
    let toggle = Step::Switch {
        switch: Switch::Background,
        state: SwitchState::Toggle,
    };
    on_a_pad(
        &mut controller,
        Macro {
            repeat: 0,
            restore: true,
            ..macro_item("macro-x", vec![toggle])
        },
    );
    press(&mut controller, 0, true, now);
    press(&mut controller, 0, false, now);

    let outputs = button(&mut controller, STOP, now);
    assert_eq!(
        requests(&outputs),
        [&Request::ToggleBackground, &Request::StopAllSounds],
        "the background as it was, then the sounds stopped"
    );
    assert_eq!(controller.macro_progress(), None);

    let later = now + Duration::from_secs(1);
    press(&mut controller, 0, true, later);
    assert!(controller.macro_progress().is_some());
    controller.set_macros(Vec::new());
    assert_eq!(controller.macro_progress(), None, "deleted while it ran");
}

#[test]
fn tells_of_a_sound_played_while_muted_for_me() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a")])]);
    controller.voicemod_event(VoicemodEvent::MuteForMe(true), now);
    press(&mut controller, 0, true, now);
    assert_eq!(controller.played_while_muted(), Some(MutedBy::Switch));

    controller.voicemod_event(VoicemodEvent::MuteForMe(false), now);
    assert_eq!(controller.played_while_muted(), None);
}

#[test]
fn tells_of_a_sound_played_with_the_volume_for_the_user_at_zero() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a")])]);
    controller.voicemod_event(VoicemodEvent::MuteForMe(false), now);
    bridge(&mut controller, [1.0, 0.0, 1.0], now);
    press(&mut controller, 0, true, now);
    assert_eq!(controller.played_while_muted(), Some(MutedBy::Volume));

    // Voicemod's slider moved up
    let up = VoicemodEvent::MixerGain {
        channel: Channel::SoundboardForMe,
        gain: 0.5,
        enabled: Some(true),
        ours: false,
    };
    controller.voicemod_event(up, now);
    assert_eq!(controller.played_while_muted(), None);
}

#[test]
fn hearing_again_with_mute_for_me_on_lets_voicemod_bring_its_volume_back() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a")])]);
    controller.voicemod_event(VoicemodEvent::MuteForMe(true), now);
    // A channel off reads as 0 whatever its slider
    bridge(&mut controller, [1.0, 0.0, 1.0], now);
    press(&mut controller, 0, true, now);
    assert_eq!(controller.played_while_muted(), Some(MutedBy::Switch));
    controller.take_outputs();
    controller.hear_sounds_again();
    assert_eq!(requests(&controller.take_outputs()), [&Request::ToggleMuteForMe]);
    assert_eq!(controller.played_while_muted(), None);
}

#[test]
fn hearing_again_with_the_volume_at_zero_sets_it_to_100() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a")])]);
    controller.voicemod_event(VoicemodEvent::MuteForMe(false), now);
    bridge(&mut controller, [1.0, 0.0, 1.0], now);
    press(&mut controller, 0, true, now);
    controller.take_outputs();
    controller.hear_sounds_again();
    let expected = Request::SetGain {
        channel: Channel::SoundboardForMe,
        gain: 1.0,
        enabled: true,
    };
    assert_eq!(requests(&controller.take_outputs()), [&expected]);
    assert_eq!(controller.played_while_muted(), None);
}

#[test]
fn a_muted_channel_read_as_zero_is_no_volume_to_step_from() {
    let (mut controller, now) = ready(three_soundboards());
    with_knob(
        &mut controller,
        KnobSettings {
            turn: TurnAction::SoundboardForMe,
            ..KnobSettings::default()
        },
    );
    controller.voicemod_event(VoicemodEvent::MuteForMe(true), now);
    bridge(&mut controller, [1.0, 0.0, 1.0], now);
    let outputs = turn(&mut controller, 1, now);
    assert!(requests(&outputs).is_empty(), "not from a guess of 0");
    assert_eq!(home_name(&controller, now), "NOT AVAILABLE");

    // Voicemod tells the slider's value once the channel changes
    let told = VoicemodEvent::MixerGain {
        channel: Channel::SoundboardForMe,
        gain: 0.09,
        enabled: Some(false),
        ours: false,
    };
    controller.voicemod_event(told, now);
    let later = now + Duration::from_secs(1);
    let outputs = turn(&mut controller, 1, later);
    let expected = Request::SetGain {
        channel: Channel::SoundboardForMe,
        gain: gain(35),
        enabled: false,
    };
    assert_eq!(requests(&outputs), [&expected]);
}

#[test]
fn mute_for_me_switched_in_voicemods_window_is_followed() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a")])]);
    controller.voicemod_event(VoicemodEvent::MuteForMe(false), now);
    bridge(&mut controller, [1.0, 1.0, 1.0], now);
    let switched = |on| VoicemodEvent::MixerGain {
        channel: Channel::SoundboardForMe,
        gain: 1.0,
        enabled: Some(on),
        ours: false,
    };
    controller.voicemod_event(switched(false), now);
    press(&mut controller, 0, true, now);
    assert_eq!(controller.played_while_muted(), Some(MutedBy::Switch));
    controller.voicemod_event(switched(true), now);
    assert_eq!(controller.played_while_muted(), None);
}

#[test]
fn a_waiting_volume_takes_mute_for_me_as_it_is_when_sent() {
    let (mut controller, now) = ready(three_soundboards());
    with_knob(
        &mut controller,
        KnobSettings {
            turn: TurnAction::SoundboardForMe,
            ..KnobSettings::default()
        },
    );
    controller.voicemod_event(VoicemodEvent::MuteForMe(false), now);
    bridge(&mut controller, [1.0, 1.0, 1.0], now);
    turn(&mut controller, -1, now);
    turn(&mut controller, -1, now + Duration::from_millis(10));
    controller.voicemod_event(VoicemodEvent::MuteForMe(true), now + Duration::from_millis(20));
    let expected = Request::SetGain {
        channel: Channel::SoundboardForMe,
        gain: gain(90),
        enabled: false,
    };
    assert_eq!(
        requests_at(&mut controller, now + Duration::from_millis(100)),
        [expected]
    );
}

#[test]
fn the_volume_notice_closed_stays_closed() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a")])]);
    controller.voicemod_event(VoicemodEvent::MuteForMe(false), now);
    bridge(&mut controller, [1.0, 0.0, 1.0], now);
    press(&mut controller, 0, true, now);
    assert_eq!(controller.played_while_muted(), Some(MutedBy::Volume));
    controller.close_volume_notice();
    assert_eq!(controller.played_while_muted(), None);
    press(&mut controller, 0, false, now);
    press(&mut controller, 0, true, now + Duration::from_secs(5));
    assert_eq!(controller.played_while_muted(), None);
}

#[test]
fn the_bridge_gone_takes_the_volume_notice_with_it() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a")])]);
    controller.voicemod_event(VoicemodEvent::MuteForMe(false), now);
    bridge(&mut controller, [1.0, 0.0, 1.0], now);
    press(&mut controller, 0, true, now);
    controller.voicemod_event(VoicemodEvent::SingleSoundStop(false), now);
    assert_eq!(controller.played_while_muted(), None);
}

#[test]
fn without_the_bridge_hearing_again_only_turns_mute_for_me_off() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a")])]);
    controller.voicemod_event(VoicemodEvent::MuteForMe(true), now);
    controller.take_outputs();
    controller.hear_sounds_again();
    assert_eq!(requests(&controller.take_outputs()), [&Request::ToggleMuteForMe]);
}

#[test]
fn the_knob_keeps_a_muted_channel_muted() {
    let (mut controller, now) = ready(three_soundboards());
    with_knob(
        &mut controller,
        KnobSettings {
            turn: TurnAction::SoundboardForMe,
            ..KnobSettings::default()
        },
    );
    controller.voicemod_event(VoicemodEvent::MuteForMe(true), now);
    bridge(&mut controller, [1.0, 1.0, 1.0], now);
    let outputs = turn(&mut controller, -1, now);
    let expected = Request::SetGain {
        channel: Channel::SoundboardForMe,
        gain: gain(95),
        enabled: false,
    };
    assert_eq!(requests(&outputs), [&expected]);
}

#[test]
fn a_sound_started_on_the_computer_counts_too() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("honk")])]);
    controller.voicemod_event(VoicemodEvent::MuteForMe(true), now);
    controller.log_input(LogInput::Opened(Vec::new()), now);
    controller.log_input(LogInput::Lines(vec![played_in_log("honk", SystemTime::now())]), now);
    assert_eq!(controller.played_while_muted(), Some(MutedBy::Switch));
}

#[test]
fn nothing_to_tell_with_the_sound_on() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a")])]);
    controller.voicemod_event(VoicemodEvent::MuteForMe(false), now);
    press(&mut controller, 0, true, now);
    assert_eq!(controller.played_while_muted(), None);
}

#[test]
fn a_locked_pad_blinks_red_and_tells_why() {
    let locked = Sound {
        enabled: false,
        ..sound("locked")
    };
    let (mut controller, now) = ready(vec![soundboard("b", vec![locked])]);
    let outputs = press(&mut controller, 0, true, now);
    assert!(requests(&outputs).is_empty());
    assert_eq!(led(&outputs, 0), Some("255,42,42,BLINK"));
    let Screen::Home { name, .. } = controller.screen(now).0 else {
        panic!("the home screen");
    };
    assert_eq!(name, ENGLISH.pro_only);

    controller.update(now + REFUSED_BLINK);
    assert_eq!(led(&controller.take_outputs(), 0), Some("90,16,16"));
}

#[test]
fn a_locked_voice_blinks_red_and_tells_why() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a")])]);
    voices(&mut controller, "dark", now);
    let locked = Content::Voice {
        id: "locked".into(),
        name: "Locked".into(),
    };
    controller.set_custom_pages(vec![own_page("mine", vec![custom_pad(locked)])]);
    controller.go_to_page(0);
    controller.update(now);
    controller.take_outputs();

    let outputs = press(&mut controller, 0, true, now);
    assert!(requests(&outputs).is_empty());
    assert_eq!(led(&outputs, 0), Some("255,42,42,BLINK"));
    let Screen::Home { name, .. } = controller.screen(now).0 else {
        panic!("the home screen");
    };
    assert_eq!(name, ENGLISH.pro_only);
}

fn logged(outputs: &[Output]) -> Vec<&str> {
    outputs
        .iter()
        .filter_map(|output| match output {
            Output::Log(message) => Some(message.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_locked_pad_goes_to_the_error_list() {
    let locked = Sound {
        enabled: false,
        name: "Explode".into(),
        ..sound("locked")
    };
    let (mut controller, now) = ready(vec![soundboard("b", vec![locked])]);
    let outputs = press(&mut controller, 0, true, now);
    assert_eq!(logged(&outputs), ["Explode is outside Voicemod's licence: not played"]);
}

#[test]
fn a_page_change_ends_the_red_blink() {
    let locked = Sound {
        enabled: false,
        ..sound("locked")
    };
    let (mut controller, now) = ready(vec![soundboard("a", vec![locked]), soundboard("b", vec![sound("b1")])]);
    press(&mut controller, 0, true, now);
    controller.go_to_page(1);
    controller.update(now);
    assert_eq!(led(&controller.take_outputs(), 0), Some("47,107,255"));
}

fn with_mode(id: &str, playback_mode: PlaybackMode) -> Sound {
    Sound {
        playback_mode,
        ..sound(id)
    }
}

#[test]
fn stop_stops_every_sound_where_voicemod_would_start_it_again() {
    let (mut controller, start) = ready(vec![soundboard("b", vec![sound("a"), sound("b")])]);
    controller.set_press_again(ByMode::from_one(PressAgain::Stop));
    press(&mut controller, 1, true, start);
    press(&mut controller, 0, true, start);
    press(&mut controller, 0, false, start);

    let again = start + Duration::from_secs(1);
    let outputs = press(&mut controller, 0, true, again);
    assert_eq!(requests(&outputs), [&Request::StopAllSounds]);
    assert!(controller.playback.is_empty());
    controller.update(again + PRESS_FLASH);
    assert_eq!(led(&controller.take_outputs(), 0), Some("47,107,255"));

    let Screen::Home { name, .. } = controller.screen(again).0 else {
        panic!("the home screen");
    };
    assert_eq!(name, "A", "the pad's sound, as when Voicemod stops it");
}

#[test]
fn restart_clicks_twice_a_sound_that_a_click_stops() {
    let toggling = with_mode("a", PlaybackMode::PlayStop);
    let (mut controller, start) = ready(vec![soundboard("b", vec![toggling])]);
    controller.set_press_again(ByMode::from_one(PressAgain::Restart));
    press(&mut controller, 0, true, start);
    press(&mut controller, 0, false, start);

    let again = start + Duration::from_secs(1);
    let outputs = press(&mut controller, 0, true, again);
    assert_eq!(requests(&outputs), [&play("a", true), &play("a", true)]);
    assert_eq!(controller.playback.state("a"), Some(State::Playing));
    assert_eq!(
        controller.playback.next_change(),
        Some(again + GUESSED_LENGTH),
        "from the start"
    );
}

#[test]
fn hold_stops_the_sounds_on_release_while_it_still_plays() {
    let (mut controller, start) = ready(vec![soundboard("b", vec![sound("a")])]);
    controller.set_press_again(ByMode::from_one(PressAgain::Hold));
    press(&mut controller, 0, true, start);
    assert_eq!(
        requests(&press(&mut controller, 0, false, start + Duration::from_millis(50))),
        [&Request::StopAllSounds],
        "at once: a sound started meanwhile would be stopped too"
    );

    // Ended by itself before the release: nothing to stop
    let later = start + Duration::from_secs(10);
    press(&mut controller, 0, true, later);
    controller.update(later + GUESSED_LENGTH);
    controller.take_outputs();
    let released = later + GUESSED_LENGTH + Duration::from_secs(1);
    assert!(requests(&press(&mut controller, 0, false, released)).is_empty());
    controller.update(released + MIN_HOLD);
    assert!(requests(&controller.take_outputs()).is_empty());
}

#[test]
fn hold_clicks_again_on_release_a_sound_that_a_click_stops() {
    let toggling = with_mode("a", PlaybackMode::PlayStop);
    let (mut controller, start) = ready(vec![soundboard("b", vec![toggling])]);
    controller.set_press_again(ByMode::from_one(PressAgain::Hold));
    press(&mut controller, 0, true, start);

    let released = start + Duration::from_secs(1);
    assert_eq!(
        requests(&press(&mut controller, 0, false, released)),
        [&play("a", true)]
    );
    assert!(controller.playback.is_empty());
}

#[test]
fn stop_leaves_a_held_sound_playing_until_pressed_again() {
    let held = with_mode("held", PlaybackMode::PlayLoopOnPress);
    let (mut controller, start) = ready(vec![soundboard("b", vec![held])]);
    controller.set_press_again(ByMode::from_one(PressAgain::Stop));
    press(&mut controller, 0, true, start);
    let released = start + Duration::from_secs(1);
    assert!(requests(&press(&mut controller, 0, false, released)).is_empty());
    controller.update(released + MIN_HOLD);
    assert!(requests(&controller.take_outputs()).is_empty());
    assert_eq!(controller.playback.state("held"), Some(State::Playing));

    let again = start + Duration::from_secs(2);
    assert_eq!(requests(&press(&mut controller, 0, true, again)), [&play("held", true)]);
    assert!(controller.playback.is_empty());
}

#[test]
fn a_pads_own_choice_comes_before_its_kinds() {
    let pausing = with_mode("music", PlaybackMode::PlayPause);
    let (mut controller, start) = ready(vec![soundboard("b", vec![pausing.clone()])]);
    controller.set_press_again(ByMode::from_one(PressAgain::Stop));
    let mut music = custom_pad(Content::Sound {
        id: "music".into(),
        name: "MUSIC".into(),
    })
    .unwrap();
    music.press_again = Some(PressAgain::Pause);
    controller.set_custom_pages(vec![own_page("mine", vec![Some(music)])]);
    controller.go_to_page(0);
    press(&mut controller, 0, true, start);
    press(&mut controller, 0, false, start);

    let again = start + Duration::from_secs(1);
    assert_eq!(
        requests(&press(&mut controller, 0, true, again)),
        [&play("music", true)]
    );
    press(&mut controller, 0, false, again);
    assert_eq!(controller.playback.state("music"), Some(State::Paused));

    // The same sound on Voicemod's own page follows the choice for every pad
    controller.go_to_page(1);
    assert_eq!(
        requests(&press(&mut controller, 0, true, again)),
        [&Request::StopAllSounds]
    );
}

#[test]
fn a_peek_tells_what_a_press_again_does() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("honk")])]);
    let peek = |controller: &mut Controller| {
        knob(controller, true, now);
        press(controller, 0, true, now);
        press(controller, 0, false, now);
        knob(controller, false, now);
        let Screen::Peek { info, note, .. } = controller.screen(now).0 else {
            panic!("a peek");
        };
        (info, note)
    };
    assert_eq!(peek(&mut controller), ("ONE-SHOT".into(), String::new()));
    controller.set_press_again(ByMode::from_one(PressAgain::Stop));
    assert_eq!(peek(&mut controller), ("PLAY/STOP".into(), "STOPS ALL".into()));
    controller.set_press_again(ByMode::from_one(PressAgain::Pause));
    assert_eq!(
        peek(&mut controller),
        ("ONE-SHOT".into(), String::new()),
        "Voicemod cannot pause it"
    );
}

#[test]
fn a_pad_that_stops_every_sound_ends_the_macro_too() {
    let sounds = vec![sound("a"), sound("b"), sound("c")];
    let (mut controller, now) = ready(vec![soundboard("board", sounds)]);
    let steps = ["a", "b"].map(|id| Step::Sound {
        id: id.into(),
        name: String::new(),
        wait_end: true,
    });
    let item = macro_item("macro-x", steps.to_vec());
    let mut stopping = custom_pad(Content::Sound {
        id: "c".into(),
        name: "C".into(),
    })
    .unwrap();
    stopping.press_again = Some(PressAgain::Stop);
    let pads = vec![custom_pad(Content::Macro { id: item.id.clone() }), Some(stopping)];
    controller.set_macros(vec![item]);
    controller.set_custom_pages(vec![own_page("mine", pads)]);
    controller.go_to_page(0);

    press(&mut controller, 0, true, now);
    press(&mut controller, 0, false, now);
    press(&mut controller, 1, true, now);
    press(&mut controller, 1, false, now);
    let again = now + Duration::from_millis(500);
    assert_eq!(
        requests(&press(&mut controller, 1, true, again)),
        [&Request::StopAllSounds]
    );
    assert!(controller.running.is_none());
    assert!(
        requests_at(&mut controller, again + Duration::from_secs(5)).is_empty(),
        "b never plays"
    );
}

#[test]
fn a_pad_follows_its_sounds_type_unless_it_has_its_own_choice() {
    let song = with_mode("song", PlaybackMode::PlayPause);
    let effect = with_mode("fx", PlaybackMode::PlayRestart);
    let (mut controller, start) = ready(vec![soundboard("b", vec![song, effect])]);
    controller.voicemod_event(VoicemodEvent::SingleSoundStop(true), start);
    controller.set_press_again(ByMode {
        play_pause: PressAgain::Stop,
        ..ByMode::default()
    });
    for pad in [0, 1] {
        press(&mut controller, pad, true, start);
        press(&mut controller, pad, false, start);
    }

    let again = start + Duration::from_secs(1);
    assert_eq!(
        requests(&press(&mut controller, 0, true, again)),
        [&Request::StopSound("song".into())]
    );
    assert_eq!(
        requests(&press(&mut controller, 1, true, again)),
        [&play("fx", true)],
        "as in Voicemod"
    );
    press(&mut controller, 0, false, again);
    press(&mut controller, 1, false, again);

    let mut own = custom_pad(Content::Sound {
        id: "song".into(),
        name: "Song".into(),
    })
    .unwrap();
    own.press_again = Some(PressAgain::Restart);
    controller.set_custom_pages(vec![own_page("mine", vec![Some(own)])]);
    controller.go_to_page(0);
    let later = again + Duration::from_secs(1);
    press(&mut controller, 0, true, later);
    press(&mut controller, 0, false, later);
    assert_eq!(
        requests(&press(&mut controller, 0, true, later + Duration::from_secs(1))),
        [&Request::StopSound("song".into()), &play("song", true)],
        "the pad's own choice first"
    );
}

#[test]
fn with_voicemods_bridge_a_pad_stops_its_sound_alone() {
    let music = with_mode("music", PlaybackMode::PlayPause);
    let (mut controller, start) = ready(vec![soundboard("b", vec![sound("a"), music])]);
    controller.voicemod_event(VoicemodEvent::SingleSoundStop(true), start);
    controller.set_press_again(ByMode::from_one(PressAgain::Stop));
    press(&mut controller, 1, true, start);
    press(&mut controller, 1, false, start);
    press(&mut controller, 0, true, start);
    press(&mut controller, 0, false, start);

    let again = start + Duration::from_millis(500);
    assert_eq!(
        requests(&press(&mut controller, 0, true, again)),
        [&Request::StopSound("a".into())]
    );
    assert_eq!(controller.playback.state("a"), None);
    assert_eq!(
        controller.playback.state("music"),
        Some(State::Playing),
        "the others play on"
    );

    controller.set_press_again(ByMode::from_one(PressAgain::Restart));
    press(&mut controller, 0, false, again);
    let restarted = again + Duration::from_millis(500);
    assert_eq!(
        requests(&press(&mut controller, 1, true, restarted)),
        [&Request::StopSound("music".into()), &play("music", true)]
    );
    assert_eq!(controller.playback.state("music"), Some(State::Playing));
}

#[test]
fn without_the_bridge_any_more_a_pad_stops_every_sound_again() {
    let (mut controller, start) = ready(vec![soundboard("b", vec![sound("a")])]);
    controller.set_press_again(ByMode::from_one(PressAgain::Stop));
    controller.voicemod_event(VoicemodEvent::SingleSoundStop(true), start);
    controller.voicemod_event(VoicemodEvent::SingleSoundStop(false), start);
    press(&mut controller, 0, true, start);
    press(&mut controller, 0, false, start);
    let again = start + Duration::from_millis(500);
    assert_eq!(
        requests(&press(&mut controller, 0, true, again)),
        [&Request::StopAllSounds]
    );
}

#[test]
fn the_end_line_after_a_single_stop_does_not_undo_a_new_press() {
    let (mut controller, start) = ready(vec![soundboard("b", vec![sound("a")])]);
    controller.voicemod_event(VoicemodEvent::SingleSoundStop(true), start);
    controller.set_press_again(ByMode::from_one(PressAgain::Stop));
    controller.log_input(LogInput::Opened(Vec::new()), start);
    let wall = SystemTime::now();
    let at = |millis| wall + Duration::from_millis(millis);
    let after = |millis| start + Duration::from_millis(millis);

    press(&mut controller, 0, true, start);
    press(&mut controller, 0, false, start);
    controller.log_input(LogInput::Lines(vec![played_in_log("a", wall)]), start);
    assert_eq!(
        requests(&press(&mut controller, 0, true, after(500))),
        [&Request::StopSound("a".into())]
    );
    press(&mut controller, 0, false, after(500));
    assert_eq!(
        requests(&press(&mut controller, 0, true, after(600))),
        [&play("a", true)]
    );

    // Voicemod logs the stop, then the sound's removal, then the new play
    let lines = vec![
        LogLine {
            at: at(505),
            kind: LineKind::Stop { id: "a".into() },
        },
        LogLine {
            at: at(515),
            kind: LineKind::End { id: "a".into() },
        },
        played_in_log("a", at(605)),
    ];
    // Read a moment later; their times are on the clock of `start`
    controller.log_input(LogInput::Lines(lines), start);
    assert_eq!(controller.playback.state("a"), Some(State::Playing));
}

fn with_knob(controller: &mut Controller, knob: KnobSettings) {
    controller.set_knob(knob, Instant::now());
}

/// Voicemod's bridge found, with these gains for the soundboard for the others, for the user and the microphone
fn bridge(controller: &mut Controller, gains: [f32; 3], now: Instant) {
    controller.voicemod_event(VoicemodEvent::SingleSoundStop(true), now);
    for (channel, gain) in Channel::ALL.into_iter().zip(gains) {
        let event = VoicemodEvent::MixerGain {
            channel,
            gain,
            enabled: None,
            ours: false,
        };
        controller.voicemod_event(event, now);
    }
}

fn tap(controller: &mut Controller, now: Instant) -> Vec<Output> {
    let mut outputs = knob(controller, true, now);
    outputs.extend(knob(controller, false, now + Duration::from_millis(50)));
    outputs
}

fn home_name(controller: &Controller, now: Instant) -> String {
    let Screen::Home { name, .. } = controller.screen(now).0 else {
        panic!("the home screen");
    };
    name
}

fn set_gain(channel: Channel, gain: f32) -> Request {
    Request::SetGain {
        channel,
        gain,
        enabled: true,
    }
}

#[test]
fn the_default_knob_is_todays_knob() {
    let sounds = (0..12).map(|i| sound(&format!("s{i}"))).collect();
    let mut soundboards = vec![soundboard("a", sounds)];
    soundboards.extend(three_soundboards());
    let (mut controller, now) = ready(soundboards);
    with_knob(&mut controller, KnobSettings::default());

    turn(&mut controller, 1, now);
    assert_eq!(controller.page().unwrap().number, 2, "a turn changes the page");
    turn(&mut controller, -1, now);

    tap(&mut controller, now);
    assert!(controller.menu.is_some(), "a tap opens the menu at once");
    tap(&mut controller, now);
    assert!(controller.menu.is_none());

    knob(&mut controller, true, now);
    turn(&mut controller, 1, now);
    assert!(
        matches!(controller.screen(now).0, Screen::Row { .. }),
        "hold and turn: the rows"
    );
    knob(&mut controller, false, now);
    assert!(controller.menu.is_none());

    knob(&mut controller, true, now);
    press(&mut controller, 0, true, now);
    assert!(
        matches!(controller.screen(now).0, Screen::Peek { .. }),
        "hold and press a pad: a peek"
    );
    press(&mut controller, 0, false, now);
    knob(&mut controller, false, now);
    assert!(controller.menu.is_none());

    let later = now + Duration::from_secs(10);
    knob(&mut controller, true, later);
    knob(&mut controller, false, later + Duration::from_secs(2));
    assert!(controller.menu.is_some(), "a long hold, let go, is a tap");
}

#[test]
fn a_turn_on_the_soundboard_for_the_others_steps_its_gain() {
    let (mut controller, now) = ready(three_soundboards());
    with_knob(
        &mut controller,
        KnobSettings {
            turn: TurnAction::SoundboardForOthers,
            ..KnobSettings::default()
        },
    );
    bridge(&mut controller, [1.0, 1.0, 1.0], now);
    let outputs = turn(&mut controller, -1, now);
    assert_eq!(requests(&outputs), [&set_gain(Channel::SoundboardForOthers, gain(95))]);
    let Screen::Level {
        label, percent, max, ..
    } = controller.screen(now).0
    else {
        panic!("the level");
    };
    assert_eq!((label.as_str(), percent, max), ("SOUNDBOARD OTHERS", 95, 100));
    assert_eq!(controller.page().unwrap().number, 1, "no page change");
    controller.update(now + PEEK_TIME);
    assert!(matches!(controller.screen(now + PEEK_TIME).0, Screen::Home { .. }));
}

#[test]
fn a_fast_turn_sends_one_volume_per_100_ms_and_the_last_value() {
    let (mut controller, now) = ready(three_soundboards());
    with_knob(
        &mut controller,
        KnobSettings {
            turn: TurnAction::SoundboardForOthers,
            ..KnobSettings::default()
        },
    );
    bridge(&mut controller, [1.0, 1.0, 1.0], now);
    let mut sent = Vec::new();
    for step in 0..10 {
        let outputs = turn(&mut controller, -1, now + Duration::from_millis(5 * step));
        sent.extend(requests(&outputs).into_iter().cloned());
    }
    assert_eq!(sent, [set_gain(Channel::SoundboardForOthers, gain(95))]);
    let due = now + Duration::from_millis(100);
    assert!(controller.next_update().is_some_and(|at| at <= due));
    assert_eq!(
        requests_at(&mut controller, due),
        [set_gain(Channel::SoundboardForOthers, gain(50))]
    );
    assert!(requests_at(&mut controller, due + Duration::from_secs(1)).is_empty());
}

#[test]
fn a_volume_step_without_the_bridge_only_tells_why() {
    let (mut controller, now) = ready(three_soundboards());
    with_knob(
        &mut controller,
        KnobSettings {
            turn: TurnAction::Microphone,
            ..KnobSettings::default()
        },
    );
    let outputs = turn(&mut controller, 1, now);
    assert!(requests(&outputs).is_empty());
    assert_eq!(home_name(&controller, now), "NOT AVAILABLE");
    assert_eq!(controller.page().unwrap().number, 1);
}

#[test]
fn the_playing_sounds_volume_starts_from_its_saved_volume() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("s1")])]);
    with_knob(
        &mut controller,
        KnobSettings {
            hold_turn: TurnAction::PlayingSound,
            ..KnobSettings::default()
        },
    );
    bridge(&mut controller, [1.0, 1.0, 1.0], now);
    let volumes = HashMap::from([("s1".to_string(), 0.3)]);
    controller.voicemod_event(VoicemodEvent::SoundVolumes(volumes), now);
    press(&mut controller, 0, true, now);
    press(&mut controller, 0, false, now);
    knob(&mut controller, true, now);
    let outputs = turn(&mut controller, 1, now);
    let expected = Request::SetSoundVolume {
        id: "s1".into(),
        volume: 0.35,
    };
    assert_eq!(requests(&outputs), [&expected]);
    let Screen::Level { label, percent, .. } = controller.screen(now).0 else {
        panic!("the level");
    };
    assert_eq!((label.as_str(), percent), ("S1", 35));
}

#[test]
fn no_sound_playing_tells_so() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("s1")])]);
    with_knob(
        &mut controller,
        KnobSettings {
            turn: TurnAction::PlayingSound,
            ..KnobSettings::default()
        },
    );
    bridge(&mut controller, [1.0, 1.0, 1.0], now);
    controller.take_outputs();
    let outputs = turn(&mut controller, 1, now);
    assert!(requests(&outputs).is_empty());
    assert_eq!(home_name(&controller, now), "NO SOUND");
}

#[test]
fn reset_puts_back_voicemods_own_values() {
    let (mut controller, now) = ready(three_soundboards());
    with_knob(
        &mut controller,
        KnobSettings {
            turn: TurnAction::SoundboardForOthers,
            tap: PressAction::ResetVolumes,
            ..KnobSettings::default()
        },
    );
    bridge(&mut controller, [gain(50), 1.0, 1.0], now);
    let outputs = turn(&mut controller, -1, now);
    assert_eq!(requests(&outputs), [&set_gain(Channel::SoundboardForOthers, gain(45))]);
    let later = now + Duration::from_secs(1);
    let outputs = tap(&mut controller, later);
    assert_eq!(requests(&outputs), [&set_gain(Channel::SoundboardForOthers, gain(50))]);
    assert_eq!(home_name(&controller, later), "VOLUMES RESET");
    assert!(controller.menu.is_none());
}

#[test]
fn a_double_tap_runs_its_macro_and_a_tap_still_opens_the_menu() {
    let (mut controller, now) = ready(three_soundboards());
    controller.set_macros(vec![macro_item("m", vec![Step::StopSounds])]);
    with_knob(
        &mut controller,
        KnobSettings {
            double_tap: PressAction::Macro { macro_id: "m".into() },
            ..KnobSettings::default()
        },
    );
    let mut outputs = tap(&mut controller, now);
    outputs.extend(tap(&mut controller, now + Duration::from_millis(200)));
    assert_eq!(requests(&outputs), [&Request::StopAllSounds], "the macro's step");
    assert!(controller.menu.is_none());

    let later = now + Duration::from_secs(5);
    tap(&mut controller, later);
    assert!(controller.menu.is_none(), "waiting for a second tap");
    let window_end = later + Duration::from_millis(50) + knob::DOUBLE_TAP;
    assert_eq!(controller.next_update(), Some(window_end));
    controller.update(window_end);
    assert!(controller.menu.is_some());
}

#[test]
fn the_menu_takes_taps_at_once_even_with_a_double_tap_set() {
    let (mut controller, now) = ready(three_soundboards());
    with_knob(
        &mut controller,
        KnobSettings {
            double_tap: PressAction::StopSounds,
            ..KnobSettings::default()
        },
    );
    tap(&mut controller, now);
    let open = now + Duration::from_secs(1);
    controller.update(open);
    assert!(controller.menu.is_some());
    turn(&mut controller, 1, open);
    let outputs = tap(&mut controller, open);
    assert!(requests(&outputs).is_empty());
    assert!(controller.menu.is_none(), "picked at once");
    assert_eq!(controller.page().unwrap().number, 2);
}

#[test]
fn a_knob_macro_that_no_longer_exists_does_nothing() {
    let (mut controller, now) = ready(three_soundboards());
    with_knob(
        &mut controller,
        KnobSettings {
            tap: PressAction::Macro {
                macro_id: "gone".into(),
            },
            ..KnobSettings::default()
        },
    );
    let outputs = tap(&mut controller, now);
    assert!(requests(&outputs).is_empty());
    assert!(controller.menu.is_none());
    assert!(controller.running.is_none());
}

#[test]
fn turning_through_voices_walks_the_favourites() {
    let now = Instant::now();
    let mut controller = Controller::default();
    let favourite = |id: &str| Voice {
        favorited: true,
        ..voice(id)
    };
    let voices = vec![voice("v1"), favourite("v2"), voice("v3"), voice("v4"), favourite("v5")];
    controller.voicemod_event(VoicemodEvent::Status(VoicemodStatus::Connected), now);
    let current = Some("v1".to_string());
    controller.voicemod_event(VoicemodEvent::Voices { voices, current }, now);
    controller.voicemod_event(VoicemodEvent::Soundboards(three_soundboards()), now);
    with_knob(
        &mut controller,
        KnobSettings {
            turn: TurnAction::Voice,
            ..KnobSettings::default()
        },
    );
    let at = |step: u64| now + Duration::from_millis(300 * step);
    let loaded: Vec<_> = (0..3)
        .flat_map(|step| {
            requests(&turn(&mut controller, 1, at(step)))
                .into_iter()
                .cloned()
                .collect::<Vec<_>>()
        })
        .collect();
    let load = |id: &str| Request::LoadVoice(id.into());
    assert_eq!(loaded, [load("v2"), load("v5"), load("v2")]);
    assert_eq!(requests(&turn(&mut controller, -1, at(3))), [&load("v5")]);
}

#[test]
fn a_fast_turn_on_the_voices_loads_only_the_last_one_in_a_while() {
    let now = Instant::now();
    let mut controller = Controller::default();
    let voices = vec![voice("v1"), voice("v2"), voice("v3"), voice("v4")];
    controller.voicemod_event(VoicemodEvent::Status(VoicemodStatus::Connected), now);
    let current = Some("v1".to_string());
    controller.voicemod_event(VoicemodEvent::Voices { voices, current }, now);
    controller.voicemod_event(VoicemodEvent::Soundboards(three_soundboards()), now);
    let knob = KnobSettings {
        turn: TurnAction::Voice,
        ..KnobSettings::default()
    };
    with_knob(&mut controller, knob);
    let at = |millis: u64| now + Duration::from_millis(millis);
    let load = |id: &str| Request::LoadVoice(id.into());
    assert_eq!(requests(&turn(&mut controller, 1, at(0))), [&load("v2")]);
    assert!(requests(&turn(&mut controller, 1, at(40))).is_empty());
    assert!(requests(&turn(&mut controller, 1, at(80))).is_empty());
    assert!(requests_at(&mut controller, at(200)).is_empty());
    assert_eq!(requests_at(&mut controller, at(250)), [load("v4")]);
}

#[test]
fn rows_on_a_plain_turn_stay_four_seconds() {
    let sounds = (0..12).map(|i| sound(&format!("s{i}"))).collect();
    let (mut controller, now) = ready(vec![soundboard("b", sounds)]);
    with_knob(
        &mut controller,
        KnobSettings {
            turn: TurnAction::Rows,
            ..KnobSettings::default()
        },
    );
    let outputs = turn(&mut controller, 1, now);
    assert_eq!(led(&outputs, 0), Some("200,200,210"));
    let Screen::Row { heading, .. } = controller.screen(now).0 else {
        panic!("a row");
    };
    assert_eq!(heading, "ROW 1/6");
    turn(&mut controller, 1, now + Duration::from_secs(1));
    let still = now + Duration::from_millis(4900);
    controller.update(still);
    assert!(
        matches!(controller.screen(still).0, Screen::Row { .. }),
        "4 s after the last step"
    );
    let gone = now + Duration::from_secs(5);
    assert!(controller.next_update().is_some_and(|at| at <= gone));
    controller.update(gone);
    assert!(matches!(controller.screen(gone).0, Screen::Home { .. }));
    assert_eq!(controller.page().unwrap().number, 1);
}

#[test]
fn every_gesture_is_reported_for_diagnostics() {
    let (mut controller, now) = ready(three_soundboards());
    let gestures = |outputs: &[Output]| -> Vec<Gesture> {
        outputs
            .iter()
            .filter_map(|output| match output {
                Output::Gesture(gesture) => Some(*gesture),
                _ => None,
            })
            .collect()
    };
    assert_eq!(gestures(&tap(&mut controller, now)), [Gesture::Tap]);
    assert_eq!(gestures(&turn(&mut controller, -1, now)), [Gesture::Turn(-1)]);
}

fn level(controller: &Controller, now: Instant) -> Option<u16> {
    match controller.screen(now).0 {
        Screen::Level { percent, .. } => Some(percent),
        _ => None,
    }
}

#[test]
fn a_quick_turn_keeps_every_step_when_voicemod_answers() {
    let (mut controller, now) = ready(three_soundboards());
    let knob = KnobSettings {
        turn: TurnAction::SoundboardForOthers,
        ..KnobSettings::default()
    };
    with_knob(&mut controller, knob);
    bridge(&mut controller, [1.0, 1.0, 1.0], now);
    let at = |millis: u64| now + Duration::from_millis(millis);
    turn(&mut controller, -1, at(0));
    turn(&mut controller, -1, at(10));
    // Voicemod tells every client of the first change while the second waits
    let echo = VoicemodEvent::MixerGain {
        channel: Channel::SoundboardForOthers,
        gain: gain(95),
        enabled: Some(true),
        ours: true,
    };
    controller.voicemod_event(echo, at(15));
    turn(&mut controller, -1, at(20));
    assert_eq!(level(&controller, at(20)), Some(85));
    assert_eq!(
        requests_at(&mut controller, at(100)),
        [set_gain(Channel::SoundboardForOthers, gain(85))]
    );
}

#[test]
fn the_menu_shows_over_a_level_or_the_rows() {
    let sounds = (0..12).map(|i| sound(&format!("s{i}"))).collect();
    let soundboards = vec![soundboard("b", sounds), soundboard("c", vec![sound("c1")])];
    for turn_action in [TurnAction::SoundboardForOthers, TurnAction::Rows] {
        let (mut controller, now) = ready(soundboards.clone());
        let knob = KnobSettings {
            turn: turn_action,
            ..KnobSettings::default()
        };
        with_knob(&mut controller, knob);
        bridge(&mut controller, [1.0, 1.0, 1.0], now);
        turn(&mut controller, 1, now);
        let later = now + Duration::from_millis(500);
        tap(&mut controller, later);
        assert!(
            matches!(
                controller.screen(later + Duration::from_millis(60)).0,
                Screen::Menu { .. }
            ),
            "{turn_action:?}"
        );
    }
}

#[test]
fn a_peek_shows_over_the_rows() {
    let sounds = (0..12).map(|i| sound(&format!("s{i}"))).collect();
    let (mut controller, now) = ready(vec![soundboard("b", sounds)]);
    let knob_settings = KnobSettings {
        turn: TurnAction::Rows,
        ..KnobSettings::default()
    };
    with_knob(&mut controller, knob_settings);
    turn(&mut controller, 1, now);
    let later = now + Duration::from_millis(500);
    knob(&mut controller, true, later);
    press(&mut controller, 3, true, later);
    assert!(matches!(controller.screen(later).0, Screen::Peek { .. }));
}

#[test]
fn a_knob_gesture_to_a_deleted_macro_does_nothing() {
    let (mut controller, now) = ready(three_soundboards());
    let steps = vec![Step::StopSounds, Step::Wait { millis: 5000 }, Step::StopSounds];
    controller.set_macros(vec![macro_item("m", steps)]);
    let knob = KnobSettings {
        tap: PressAction::Macro {
            macro_id: "gone".into(),
        },
        ..KnobSettings::default()
    };
    with_knob(&mut controller, knob);
    controller.try_macro("m", now);
    controller.update(now);
    assert!(controller.running.is_some());
    tap(&mut controller, now + Duration::from_millis(100));
    assert!(controller.running.is_some(), "the macro running goes on");
}

#[test]
fn the_volume_of_the_sound_playing_skips_a_paused_one() {
    let effect = Sound {
        playback_mode: PlaybackMode::PlayStop,
        ..sound("loop")
    };
    let song = Sound {
        playback_mode: PlaybackMode::PlayPause,
        ..sound("song")
    };
    let (mut controller, now) = ready(vec![soundboard("b", vec![effect, song])]);
    let knob = KnobSettings {
        turn: TurnAction::PlayingSound,
        ..KnobSettings::default()
    };
    with_knob(&mut controller, knob);
    bridge(&mut controller, [1.0, 1.0, 1.0], now);
    let volumes = HashMap::from([("song".to_string(), 0.5), ("loop".to_string(), 0.5)]);
    controller.voicemod_event(VoicemodEvent::SoundVolumes(volumes), now);
    let at = |millis: u64| now + Duration::from_millis(millis);
    for (pad, millis) in [(0, 0), (1, 100), (1, 1000)] {
        press(&mut controller, pad, true, at(millis));
        press(&mut controller, pad, false, at(millis + 50));
    }
    assert_eq!(controller.playback.state("song"), Some(State::Paused));
    let outputs = turn(&mut controller, 1, at(1200));
    let expected = Request::SetSoundVolume {
        id: "loop".into(),
        volume: 0.55,
    };
    assert_eq!(requests(&outputs), [&expected]);
    assert_eq!(level(&controller, at(1200)), Some(55));
}

fn volume_reads(outputs: &[Output]) -> usize {
    requests(outputs)
        .into_iter()
        .filter(|request| **request == Request::ReadSoundVolumes)
        .count()
}

#[test]
fn the_sounds_volumes_are_read_only_for_a_gesture_that_needs_them() {
    let (mut controller, now) = ready(three_soundboards());
    controller.set_knob(KnobSettings::default(), now);
    bridge(&mut controller, [1.0, 1.0, 1.0], now);
    assert_eq!(volume_reads(&controller.take_outputs()), 0);
    let knob = KnobSettings {
        hold_turn: TurnAction::PlayingSound,
        ..KnobSettings::default()
    };
    controller.set_knob(knob, now);
    assert_eq!(volume_reads(&controller.take_outputs()), 1);
}

#[test]
fn the_sounds_volumes_are_read_at_most_once_a_minute() {
    let effect = Sound {
        playback_mode: PlaybackMode::PlayStop,
        ..sound("loop")
    };
    let (mut controller, now) = ready(vec![soundboard("b", vec![effect])]);
    let at = |seconds: u64| now + Duration::from_secs(seconds);
    let knob = KnobSettings {
        turn: TurnAction::PlayingSound,
        ..KnobSettings::default()
    };
    controller.set_knob(knob, at(0));
    assert_eq!(volume_reads(&controller.take_outputs()), 0, "not without the bridge");
    bridge(&mut controller, [1.0, 1.0, 1.0], at(0));
    controller.voicemod_event(
        VoicemodEvent::Soundboards(vec![soundboard("b", vec![sound("loop")])]),
        at(1),
    );
    assert_eq!(volume_reads(&controller.take_outputs()), 1);

    // Voicemod restarted soon after: the read waits for its minute
    controller.voicemod_event(VoicemodEvent::SingleSoundStop(false), at(5));
    bridge(&mut controller, [1.0, 1.0, 1.0], at(10));
    assert_eq!(volume_reads(&controller.take_outputs()), 0);
    controller.update(at(59));
    assert_eq!(volume_reads(&controller.take_outputs()), 0);
    controller.update(at(60));
    assert_eq!(volume_reads(&controller.take_outputs()), 1);

    // Voicemod's answer did not hold the sound: a step asks again, in its minute
    press(&mut controller, 0, true, at(70));
    press(&mut controller, 0, false, at(70));
    let outputs = turn(&mut controller, 1, at(71));
    assert_eq!(volume_reads(&outputs), 0);
    controller.update(at(119));
    assert_eq!(volume_reads(&controller.take_outputs()), 0);
    controller.update(at(120));
    assert_eq!(volume_reads(&controller.take_outputs()), 1);
    controller.update(at(500));
    assert_eq!(volume_reads(&controller.take_outputs()), 0, "nothing asks again");
}

fn walking_voices(knob: KnobSettings) -> (Controller, Instant) {
    let now = Instant::now();
    let mut controller = Controller::default();
    let voices = ["v1", "v2", "v3", "v4", "v5"].map(voice).to_vec();
    controller.voicemod_event(VoicemodEvent::Status(VoicemodStatus::Connected), now);
    let current = Some("v1".to_string());
    controller.voicemod_event(VoicemodEvent::Voices { voices, current }, now);
    controller.voicemod_event(VoicemodEvent::Soundboards(three_soundboards()), now);
    controller.set_knob(knob, now);
    (controller, now)
}

#[test]
fn a_voice_loaded_right_after_a_fast_turn_stays() {
    let knob = KnobSettings {
        turn: TurnAction::Voice,
        tap: PressAction::RandomVoice,
        ..KnobSettings::default()
    };
    let (mut controller, now) = walking_voices(knob);
    let at = |millis: u64| now + Duration::from_millis(millis);
    turn(&mut controller, 1, at(0));
    turn(&mut controller, 1, at(40));
    let random = requests(&tap(&mut controller, at(100))).len();
    assert_eq!(random, 1, "a random voice loads");
    assert!(
        requests_at(&mut controller, at(300)).is_empty(),
        "the turn's waiting voice does not follow"
    );
}

#[test]
fn voicemod_confirming_an_earlier_voice_keeps_the_knobs_walk() {
    let knob = KnobSettings {
        turn: TurnAction::Voice,
        ..KnobSettings::default()
    };
    let (mut controller, now) = walking_voices(knob);
    let at = |millis: u64| now + Duration::from_millis(millis);
    turn(&mut controller, 1, at(0));
    turn(&mut controller, 1, at(40));
    controller.voicemod_event(VoicemodEvent::CurrentVoice("v2".into()), at(60));
    turn(&mut controller, 1, at(80));
    assert_eq!(requests_at(&mut controller, at(250)), [Request::LoadVoice("v4".into())]);
    // Sent, then the older confirmation comes late: the next step still goes on from v4
    controller.voicemod_event(VoicemodEvent::CurrentVoice("v3".into()), at(300));
    let outputs = turn(&mut controller, 1, at(600));
    assert_eq!(requests(&outputs), [&Request::LoadVoice("v5".into())]);
}

#[test]
fn the_windows_knob_works_as_the_boards() {
    let (mut controller, now) = ready(three_soundboards());
    controller.window_knob_turned(1, now);
    controller.update(now);
    assert_eq!(controller.page().unwrap().number, 2, "a turn changes the page");
    let later = now + Duration::from_millis(50);
    controller.window_knob_pressed(true, now);
    controller.window_knob_pressed(false, later);
    controller.update(later);
    assert!(controller.menu.is_some(), "a tap opens the menu");
}

fn wheel_on(hold_turn: TurnAction, tap: PressAction) -> (Controller, Instant) {
    let (mut controller, now) = ready_without_overlay(three_soundboards());
    controller.set_knob(
        KnobSettings {
            hold_turn,
            tap,
            ..KnobSettings::default()
        },
        now,
    );
    controller.set_wheel(crate::wheel::WheelSettings {
        slices: vec![
            PressAction::MuteMic,
            PressAction::Page { page: "c".into() },
            PressAction::StopSounds,
        ],
        ..crate::wheel::WheelSettings::default()
    });
    (controller, now)
}

fn wheel_in(outputs: &[Output]) -> Option<Option<crate::overlay::Wheel>> {
    outputs.iter().rev().find_map(|output| match output {
        Output::Overlay(Some(crate::overlay::Content::Wheel(wheel))) => Some(Some(wheel.clone())),
        Output::Overlay(_) => Some(None),
        _ => None,
    })
}

#[test]
fn turning_held_opens_the_wheel_and_letting_go_picks() {
    let (mut controller, now) = wheel_on(TurnAction::Wheel, PressAction::Menu);
    knob(&mut controller, true, now);
    let outputs = turn(&mut controller, 1, now);
    let Some(Some(wheel)) = wheel_in(&outputs) else {
        panic!("the wheel");
    };
    assert_eq!(wheel.selected, Some(0));
    assert_eq!(
        wheel.slices.iter().map(|slice| slice.name.as_str()).collect::<Vec<_>>(),
        ["Mute mic", "C", "Stop sounds"]
    );
    assert_eq!(wheel.centre, "Mute mic");
    turn(&mut controller, 1, now);
    let outputs = knob(&mut controller, false, now);
    assert_eq!(controller.page, 2, "the page slice");
    assert!(requests(&outputs).is_empty());
    assert_eq!(wheel_in(&outputs), Some(None), "closed");
}

#[test]
fn letting_go_at_the_centre_does_nothing() {
    let (mut controller, now) = wheel_on(TurnAction::Wheel, PressAction::Menu);
    knob(&mut controller, true, now);
    turn(&mut controller, 1, now);
    let outputs = turn(&mut controller, -1, now);
    assert_eq!(wheel_in(&outputs).flatten().map(|wheel| wheel.selected), Some(None));
    let outputs = knob(&mut controller, false, now);
    assert!(requests(&outputs).is_empty());
    assert_eq!(controller.page, 0);
    assert!(controller.menu.is_none(), "no tap either");
    assert!(overlay_in(&outputs).flatten().is_none(), "nor the page's names");
}

#[test]
fn a_tap_opens_the_wheel_the_turns_pick_and_a_tap_does_it() {
    let (mut controller, now) = wheel_on(TurnAction::Rows, PressAction::Wheel);
    knob(&mut controller, true, now);
    let outputs = knob(&mut controller, false, now);
    assert_eq!(wheel_in(&outputs).flatten().map(|wheel| wheel.selected), Some(None));
    assert!(matches!(controller.screen(now).0, Screen::Wheel { selected: None, .. }));
    turn(&mut controller, -1, now);
    let (screen, _) = controller.screen(now);
    let Screen::Wheel { name, counter, .. } = screen else {
        panic!("the wheel on the display");
    };
    assert_eq!((name.as_str(), counter.as_str()), ("Stop sounds", "3/3"));
    knob(&mut controller, true, now);
    let outputs = knob(&mut controller, false, now);
    assert_eq!(requests(&outputs), [&Request::StopAllSounds]);
    assert!(controller.wheel.is_none());
}

#[test]
fn a_wheel_opened_by_a_tap_closes_by_itself() {
    let (mut controller, now) = wheel_on(TurnAction::Rows, PressAction::Wheel);
    knob(&mut controller, true, now);
    knob(&mut controller, false, now);
    turn(&mut controller, 1, now + Duration::from_secs(5));
    // Its time starts again at each turn
    controller.update(now + Duration::from_secs(12));
    assert!(controller.wheel.is_some());
    controller.update(now + Duration::from_secs(13));
    assert!(controller.wheel.is_none());
    assert!(
        controller
            .take_outputs()
            .iter()
            .all(|output| !matches!(output, Output::Voicemod(_)))
    );
}

#[test]
fn the_settings_preview_takes_no_gesture() {
    let (mut controller, now) = wheel_on(TurnAction::Wheel, PressAction::Menu);
    controller.preview_wheel(now);
    controller.update(now);
    let outputs = controller.take_outputs();
    assert_eq!(wheel_in(&outputs).flatten().map(|wheel| wheel.selected), Some(Some(0)));
    knob(&mut controller, true, now);
    let outputs = knob(&mut controller, false, now);
    assert!(requests(&outputs).is_empty(), "the first slice is not done");
    assert!(controller.menu.is_some(), "the tap does what it is set to");
    assert!(controller.wheel.is_none());
}

#[test]
fn the_wheel_closes_when_voicemod_goes() {
    let (mut controller, now) = wheel_on(TurnAction::Wheel, PressAction::Menu);
    knob(&mut controller, true, now);
    turn(&mut controller, 1, now);
    controller.voicemod_event(VoicemodEvent::Status(VoicemodStatus::Unreachable), now);
    assert!(controller.wheel.is_none());
}

#[test]
fn the_wheel_goes_with_the_board() {
    let (mut controller, now) = wheel_on(TurnAction::Wheel, PressAction::Menu);
    knob(&mut controller, true, now);
    turn(&mut controller, 1, now);
    controller.set_board_connected(false, now);
    assert!(controller.wheel.is_none());
}

/// A song that loops and stops the other sounds, and a horn that stops them too, as Voicemod's own, on a time line
fn song_and_horn() -> (Controller, Instant) {
    let horn = Sound {
        stop_other_sounds: true,
        ..sound("horn")
    };
    let (mut controller, now) = ready(vec![soundboard("b", vec![song(), horn])]);
    bridge(&mut controller, [1.0, 1.0, 1.0], now);
    let timeline = Timeline {
        clips: vec![clip("song", 0, 0), clip("horn", 1, 500)],
        actions: Vec::new(),
    };
    on_a_pad(
        &mut controller,
        Macro {
            timeline: Some(timeline),
            ..macro_item("macro-s", Vec::new())
        },
    );
    controller.take_outputs();
    (controller, now)
}

fn song() -> Sound {
    Sound {
        looped: true,
        stop_other_sounds: true,
        ..sound("song")
    }
}

fn override_of(id: &str, on: bool) -> Request {
    Request::OverrideSoundOptions {
        id: id.into(),
        stops_others: Some(on),
        mutes_others: None,
        loops: (id == "song").then_some(on),
        restore: on,
    }
}

fn applied(controller: &mut Controller, sound: &str, now: Instant) {
    let event = VoicemodEvent::OptionsApplied {
        sound: sound.into(),
        changed: true,
    };
    controller.voicemod_event(event, now);
}

#[test]
fn a_macro_s_sounds_play_over_each_other_once_each() {
    let (mut controller, now) = song_and_horn();
    let at = |millis| now + Duration::from_millis(millis);

    let outputs = press(&mut controller, 0, true, now);
    press(&mut controller, 0, false, now);
    let asked: Vec<&Request> = requests(&outputs);
    assert!(asked.contains(&&override_of("song", false)) && asked.contains(&&override_of("horn", false)));
    assert!(
        asked.contains(&&play("song", true)),
        "the first plays at once: nothing to stop yet"
    );
    assert!(
        outputs
            .iter()
            .any(|output| matches!(output, Output::Overrides(kept) if kept.len() == 2))
    );
    applied(&mut controller, "song", at(100));
    assert_eq!(
        requests_at(&mut controller, at(500)),
        [],
        "the horn waits, as it would stop the song"
    );
    applied(&mut controller, "horn", at(600));
    assert_eq!(requests_at(&mut controller, at(600)), [play("horn", true)]);

    // Pressed again, the macro stops with its sounds, and their options go back
    let requests: Vec<Request> = requests(&press(&mut controller, 0, true, at(800)))
        .into_iter()
        .cloned()
        .collect();
    press(&mut controller, 0, false, at(800));
    for expected in [
        Request::StopSound("song".into()),
        Request::StopSound("horn".into()),
        override_of("song", true),
        override_of("horn", true),
    ] {
        assert!(requests.contains(&expected), "{expected:?} in {requests:?}");
    }
}

#[test]
fn a_clip_waits_a_second_at_most_for_its_options() {
    let (mut controller, now) = song_and_horn();
    let at = |millis| now + Duration::from_millis(millis);
    press(&mut controller, 0, true, now);
    assert_eq!(requests_at(&mut controller, at(500)), []);
    assert!(
        controller
            .next_update()
            .is_some_and(|next| next <= now + overrides::OVERRIDE_WAIT)
    );
    assert_eq!(
        requests_at(
            &mut controller,
            now + overrides::OVERRIDE_WAIT - Duration::from_millis(1)
        ),
        []
    );
    assert_eq!(
        requests_at(&mut controller, now + overrides::OVERRIDE_WAIT),
        [play("horn", true)]
    );
}

#[test]
fn a_looping_sound_plays_once_whatever_its_options() {
    let (mut controller, now) = song_and_horn();
    controller.file_lengths.insert("song".into(), Duration::from_secs(3));
    controller.playback.learn_length("song", Duration::from_secs(60));
    press(&mut controller, 0, true, now);
    let at = |millis| now + Duration::from_millis(millis);
    applied(&mut controller, "horn", at(100));
    requests_at(&mut controller, at(500));
    assert_eq!(
        requests_at(&mut controller, at(3000)),
        [Request::StopSound("song".into())],
        "cut at its length"
    );
}

#[test]
fn options_left_changed_go_back_once_the_bridge_is_there() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![song()])]);
    let own = OwnOptions {
        stops_others: true,
        mutes_others: false,
        loops: true,
    };
    controller.left_overrides([("song".to_string(), own)].into());
    assert_eq!(requests_at(&mut controller, now), [], "not without the bridge");
    bridge(&mut controller, [1.0, 1.0, 1.0], now);
    let outputs = {
        controller.update(now);
        controller.take_outputs()
    };
    assert!(requests(&outputs).contains(&&override_of("song", true)));
    applied(&mut controller, "song", now);
    let outputs = controller.take_outputs();
    assert!(
        outputs
            .iter()
            .any(|output| matches!(output, Output::Overrides(kept) if kept.is_empty()))
    );
}

#[test]
fn the_editor_s_stop_stops_the_macro_s_sounds() {
    let (mut controller, now) = song_and_horn();
    controller.try_macro("macro-s", now);
    controller.update(now);
    controller.take_outputs();
    controller.stop_macro(now);
    controller.update(now);
    let outputs = controller.take_outputs();
    assert_eq!(
        requests(&outputs)[0],
        &Request::StopSound("song".into()),
        "then the options go back"
    );
}

#[test]
fn a_sound_that_stops_the_others_leaves_the_macro_s_others_playing() {
    // Thunder cut at 3 s, a horse that stops the others over it at 1 s
    let horse = Sound {
        stop_other_sounds: true,
        ..sound("horse")
    };
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("thunder"), horse])]);
    bridge(&mut controller, [1.0, 1.0, 1.0], now);
    let timeline = Timeline {
        clips: vec![
            Clip {
                cut: Some(3000),
                ..clip("thunder", 0, 0)
            },
            clip("horse", 1, 1000),
        ],
        actions: Vec::new(),
    };
    on_a_pad(
        &mut controller,
        Macro {
            timeline: Some(timeline),
            ..macro_item("macro-h", Vec::new())
        },
    );
    controller.playback.learn_length("thunder", Duration::from_secs(10));
    let at = |millis| now + Duration::from_millis(millis);
    press(&mut controller, 0, true, now);
    applied(&mut controller, "horse", at(150));
    assert_eq!(requests_at(&mut controller, at(1000)), [play("horse", true)]);
    assert_eq!(
        requests_at(&mut controller, at(3000)),
        [Request::StopSound("thunder".into())],
        "the thunder still plays: its cut comes"
    );
}

#[test]
fn options_put_back_are_kept_until_voicemod_confirms_them() {
    let (mut controller, now) = song_and_horn();
    let at = |millis| now + Duration::from_millis(millis);
    press(&mut controller, 0, true, now);
    press(&mut controller, 0, false, now);
    applied(&mut controller, "song", at(100));
    applied(&mut controller, "horn", at(100));
    press(&mut controller, 0, true, at(200));
    press(&mut controller, 0, false, at(200));
    // Not confirmed: sent again
    assert!(requests_at(&mut controller, at(5199)).is_empty());
    let again = requests_at(&mut controller, at(5200));
    assert!(again.contains(&override_of("song", true)) && again.contains(&override_of("horn", true)));
    applied(&mut controller, "song", at(5300));
    let outputs = controller.take_outputs();
    assert!(
        outputs
            .iter()
            .any(|output| matches!(output, Output::Overrides(kept) if kept.len() == 1)),
        "the song is put back"
    );
}

fn refused(controller: &mut Controller, sound: &str, now: Instant) {
    let event = VoicemodEvent::OptionsApplied {
        sound: sound.into(),
        changed: false,
    };
    controller.voicemod_event(event, now);
}

fn kept_in(outputs: &[Output]) -> Option<&HashMap<String, OwnOptions>> {
    outputs.iter().rev().find_map(|output| match output {
        Output::Overrides(kept) => Some(kept),
        _ => None,
    })
}

#[test]
fn options_never_confirmed_put_back_are_given_up() {
    let (mut controller, now) = song_and_horn();
    let at = |millis| now + Duration::from_millis(millis);
    press(&mut controller, 0, true, now);
    press(&mut controller, 0, false, now);
    applied(&mut controller, "song", at(100));
    applied(&mut controller, "horn", at(100));
    press(&mut controller, 0, true, at(200));
    press(&mut controller, 0, false, at(200));
    assert!(requests_at(&mut controller, at(5200)).contains(&override_of("song", true)));
    assert!(requests_at(&mut controller, at(10200)).contains(&override_of("song", true)));
    controller.update(at(15200));
    let outputs = controller.take_outputs();
    assert!(requests(&outputs).is_empty(), "sent three times");
    assert!(kept_in(&outputs).is_some_and(HashMap::is_empty));
    assert!(requests_at(&mut controller, at(30000)).is_empty());
}

#[test]
fn options_of_a_sound_gone_from_voicemod_are_given_up() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("other")])]);
    let own = OwnOptions {
        stops_others: true,
        mutes_others: false,
        loops: true,
    };
    controller.left_overrides([("song".to_string(), own)].into());
    bridge(&mut controller, [1.0, 1.0, 1.0], now);
    controller.update(now);
    let outputs = controller.take_outputs();
    assert!(!requests(&outputs).contains(&&override_of("song", true)));
    assert!(kept_in(&outputs).is_some_and(HashMap::is_empty));
}

#[test]
fn an_answer_counts_for_the_change_it_answers() {
    let (mut controller, now) = song_and_horn();
    let at = |millis| now + Duration::from_millis(millis);
    press(&mut controller, 0, true, now);
    press(&mut controller, 0, false, now);
    applied(&mut controller, "song", at(100));
    applied(&mut controller, "horn", at(100));
    // Stopped, then started again before the song's options are back
    press(&mut controller, 0, true, at(200));
    press(&mut controller, 0, false, at(200));
    press(&mut controller, 0, true, at(300));
    press(&mut controller, 0, false, at(300));
    refused(&mut controller, "song", at(350));
    assert_eq!(
        kept_in(&controller.take_outputs()),
        None,
        "putting back was refused, turning off is still on its way"
    );
    applied(&mut controller, "song", at(400));
    let outputs = press(&mut controller, 0, true, at(500));
    assert!(
        requests(&outputs).contains(&&override_of("song", true)),
        "still known as changed, it goes back"
    );
}

#[test]
fn a_late_answer_to_turning_off_does_not_confirm_putting_back() {
    let (mut controller, now) = song_and_horn();
    let at = |millis| now + Duration::from_millis(millis);
    press(&mut controller, 0, true, now);
    press(&mut controller, 0, false, now);
    let outputs = press(&mut controller, 0, true, at(1600));
    press(&mut controller, 0, false, at(1600));
    assert!(requests(&outputs).contains(&&override_of("song", true)));
    applied(&mut controller, "song", at(1700));
    assert_eq!(
        kept_in(&controller.take_outputs()),
        None,
        "the answer to turning it off"
    );
    applied(&mut controller, "song", at(1800));
    assert!(kept_in(&controller.take_outputs()).is_some_and(|kept| !kept.contains_key("song")));
}

#[test]
fn a_clip_waits_while_a_sound_playing_would_mute_it() {
    let bell = Sound {
        mute_other_sounds: true,
        ..sound("bell")
    };
    let (mut controller, now) = ready(vec![soundboard("b", vec![bell, sound("voice")])]);
    bridge(&mut controller, [1.0, 1.0, 1.0], now);
    let timeline = Timeline {
        clips: vec![clip("bell", 0, 0), clip("voice", 1, 300)],
        actions: Vec::new(),
    };
    on_a_pad(
        &mut controller,
        Macro {
            timeline: Some(timeline),
            ..macro_item("macro-b", Vec::new())
        },
    );
    controller.take_outputs();
    let at = |millis| now + Duration::from_millis(millis);
    let outputs = press(&mut controller, 0, true, now);
    assert!(requests(&outputs).contains(&&play("bell", true)));
    assert_eq!(requests_at(&mut controller, at(300)), [], "the bell would mute it");
    applied(&mut controller, "bell", at(400));
    assert_eq!(requests_at(&mut controller, at(400)), [play("voice", true)]);
}

#[test]
fn options_put_back_hold_only_until_a_read_shows_them() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![song()])]);
    let own = OwnOptions {
        stops_others: true,
        mutes_others: false,
        loops: true,
    };
    controller.left_overrides([("song".to_string(), own)].into());
    bridge(&mut controller, [1.0, 1.0, 1.0], now);
    controller.update(now);
    applied(&mut controller, "song", now);
    let read = |controller: &mut Controller, song: Sound| {
        controller.voicemod_event(VoicemodEvent::Soundboards(vec![soundboard("b", vec![song])]), now);
        controller.sounds["song"].looped
    };
    assert!(
        read(&mut controller, sound("song")),
        "a read asked before, its options not back yet"
    );
    assert!(read(&mut controller, song()));
    assert!(
        !read(&mut controller, sound("song")),
        "the user then turned the loop off in Voicemod"
    );
}

#[test]
fn the_time_line_keeps_its_distances_after_a_clip_waited() {
    let (mut controller, now) = song_and_horn();
    let timeline = Timeline {
        clips: vec![clip("song", 0, 0), clip("horn", 1, 500), clip("fx", 2, 900)],
        actions: Vec::new(),
    };
    controller.voicemod_event(
        VoicemodEvent::Soundboards(vec![soundboard(
            "b",
            vec![
                song(),
                Sound {
                    stop_other_sounds: true,
                    ..sound("horn")
                },
                sound("fx"),
            ],
        )]),
        now,
    );
    on_a_pad(
        &mut controller,
        Macro {
            timeline: Some(timeline),
            ..macro_item("macro-s", Vec::new())
        },
    );
    controller.take_outputs();
    let at = |millis| now + Duration::from_millis(millis);
    press(&mut controller, 0, true, now);
    assert!(
        requests_at(&mut controller, at(900)).is_empty(),
        "the horn waits, the fx behind it"
    );
    applied(&mut controller, "horn", at(1000));
    assert_eq!(requests_at(&mut controller, at(1000)), [play("horn", true)]);
    assert!(
        requests_at(&mut controller, at(1399)).is_empty(),
        "400 ms after the horn, as drawn"
    );
    assert_eq!(requests_at(&mut controller, at(1400)), [play("fx", true)]);
}

fn screen_display_in(outputs: &[Output]) -> Option<bool> {
    outputs.iter().rev().find_map(|output| match output {
        Output::ScreenDisplay(shown) => Some(*shown),
        _ => None,
    })
}

#[test]
fn the_display_shows_on_screen_with_the_soundboard_menu_and_a_moment_after() {
    let (mut controller, now) = ready(three_soundboards());
    controller.set_screen_display(crate::settings::ScreenDisplaySettings {
        when: crate::settings::ShowDisplay::OnActions,
        ..crate::settings::ScreenDisplaySettings::default()
    });
    controller.update(now);
    assert_eq!(screen_display_in(&controller.take_outputs()), None, "not yet");
    knob(&mut controller, true, now);
    let outputs = knob(&mut controller, false, now);
    assert_eq!(screen_display_in(&outputs), Some(true), "the menu opened");
    turn(&mut controller, 1, now);
    knob(&mut controller, true, now);
    knob(&mut controller, false, now);
    let later = now + Duration::from_millis(1499);
    controller.update(later);
    assert_eq!(
        screen_display_in(&controller.take_outputs()),
        None,
        "still, to see the page picked"
    );
    controller.update(now + Duration::from_millis(1500));
    assert_eq!(screen_display_in(&controller.take_outputs()), Some(false));
}

#[test]
fn the_display_on_screen_always_never_or_for_a_preview() {
    let (mut controller, now) = ready(three_soundboards());
    controller.set_screen_display(crate::settings::ScreenDisplaySettings {
        when: crate::settings::ShowDisplay::Always,
        ..crate::settings::ScreenDisplaySettings::default()
    });
    controller.update(now);
    assert_eq!(screen_display_in(&controller.take_outputs()), Some(true));
    controller.set_screen_display(crate::settings::ScreenDisplaySettings::default());
    controller.update(now);
    assert_eq!(
        screen_display_in(&controller.take_outputs()),
        Some(false),
        "never, by default"
    );
    controller.preview_screen_display(now);
    controller.update(now);
    assert_eq!(
        screen_display_in(&controller.take_outputs()),
        Some(true),
        "for the preview"
    );
    controller.update(now + Duration::from_secs(3));
    assert_eq!(screen_display_in(&controller.take_outputs()), Some(false));
}

#[test]
fn a_pad_of_one_s_own_breathes_or_blinks_at_rest_as_picked() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a"), sound("b")])]);
    let pad = |id: &str, animation| CustomPad {
        animation,
        colour: Some([0, 200, 150]),
        ..custom_pad(Content::Sound {
            id: id.into(),
            name: String::new(),
        })
        .unwrap()
    };
    controller.set_custom_pages(vec![own_page(
        "mine",
        vec![Some(pad("a", Effect::Breathe)), Some(pad("b", Effect::Blink))],
    )]);
    controller.go_to_page(0);
    controller.update(now);
    assert_eq!(
        (controller.pad_led(0, now), controller.pad_led(1, now)),
        (
            Led {
                color: [0, 200, 150],
                effect: Effect::Breathe
            },
            Led {
                color: [0, 200, 150],
                effect: Effect::Blink
            }
        )
    );
}
