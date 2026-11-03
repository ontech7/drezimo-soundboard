use super::overlay::OVERLAY_TIME;
use super::*;
use crate::custom_pages::{Content, CustomPad};
use crate::display::screens::{self, FRAME, Progress, Screen, SoundState};
use crate::macros::{Again, Macro, Step, Switch, SwitchState};
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
    turn(&mut controller, 1, now);
    knob(&mut controller, true, now);
    knob(&mut controller, false, now);
    assert_eq!(controller.page().unwrap().name, "C");
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

fn overlay_in(outputs: &[Output]) -> Option<&Option<Overlay>> {
    outputs.iter().rev().find_map(|output| match output {
        Output::Overlay(overlay) => Some(overlay),
        _ => None,
    })
}

/// A ready controller whose overlay, shown when the board connected, has gone.
fn ready_without_overlay(soundboards: Vec<Soundboard>) -> (Controller, Instant) {
    let (mut controller, start) = ready(soundboards);
    let now = start + OVERLAY_TIME;
    controller.update(now);
    assert_eq!(overlay_in(&controller.take_outputs()), Some(&None));
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
    assert_eq!(overlay_in(&controller.take_outputs()), Some(&None));
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
    assert_ne!(overlay_in(&controller.take_outputs()), Some(&None), "still shown");

    press(&mut controller, 0, true, later);
    press(&mut controller, 0, false, later);
    knob(&mut controller, false, later);
    controller.update(later + OVERLAY_TIME - Duration::from_millis(1));
    assert_ne!(overlay_in(&controller.take_outputs()), Some(&None), "still shown");
    controller.update(later + OVERLAY_TIME);
    assert_eq!(overlay_in(&controller.take_outputs()), Some(&None));
}

#[test]
fn no_overlay_without_the_board() {
    let (mut controller, now) = ready(three_soundboards());
    controller.set_board_connected(false, now);
    controller.update(now);
    assert_eq!(overlay_in(&controller.take_outputs()), Some(&None));
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
    assert_eq!(controller.board_view(now).unwrap().page, 2);
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
    }
}

#[test]
fn the_users_own_pages_come_first_and_follow_the_editor() {
    let (mut controller, _) = ready(three_soundboards());
    controller.go_to_page(1);
    let page = own_page("mine", vec![custom_pad(action(Action::StopSounds))]);
    controller.set_custom_pages(vec![page]);
    assert_eq!(controller.page().unwrap().name, "B", "the page shown stays");

    controller.go_to_page(0);
    assert_eq!(controller.page().unwrap().name, "MINE");
    controller.set_custom_pages(Vec::new());
    assert_eq!(
        controller.page().unwrap().name,
        "A",
        "a page deleted gives way to the first"
    );
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
        again: Again::Stop,
        repeat: 1,
        restore: false,
        steps,
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
    assert!(controller.played_while_muted());

    controller.voicemod_event(VoicemodEvent::MuteForMe(false), now);
    assert!(!controller.played_while_muted());
}

#[test]
fn a_sound_started_on_the_computer_counts_too() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("honk")])]);
    controller.voicemod_event(VoicemodEvent::MuteForMe(true), now);
    controller.log_input(LogInput::Opened(Vec::new()), now);
    controller.log_input(LogInput::Lines(vec![played_in_log("honk", SystemTime::now())]), now);
    assert!(controller.played_while_muted());
}

#[test]
fn nothing_to_tell_with_the_sound_on() {
    let (mut controller, now) = ready(vec![soundboard("b", vec![sound("a")])]);
    controller.voicemod_event(VoicemodEvent::MuteForMe(false), now);
    press(&mut controller, 0, true, now);
    assert!(!controller.played_while_muted());
}
