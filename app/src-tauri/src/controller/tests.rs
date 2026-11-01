use super::*;
use crate::display::screens::Screen;
use crate::soundboard::tests::{sound, soundboard};
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
    let kind = LineKind::Play {
        id: id.into(),
        mode: PlaybackMode::PlayRestart,
        repeat: false,
        paused: false,
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
    let (mut controller, now) = ready(three_soundboards());
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
    let Screen::Row { heading, lines, .. } = controller.screen(now).0 else {
        panic!("a row");
    };
    assert_eq!(heading, "ROW 2/6");
    assert_eq!(lines[0], "1 S6");

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
    let Screen::Home { name, right, .. } = controller.screen(start).0 else {
        panic!("the home screen");
    };
    assert_eq!((name.as_str(), right.as_str()), ("HONK", "3.0s"));
    assert!(
        controller
            .next_update()
            .is_some_and(|at| at <= start + Duration::from_millis(100))
    );

    let later = start + Duration::from_millis(1500);
    controller.update(later);
    let Screen::Home { right, .. } = controller.screen(later).0 else {
        panic!("the home screen");
    };
    assert_eq!(right, "1.5s");
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
    let Screen::Voicemod { message } = controller.screen(now).0 else {
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
