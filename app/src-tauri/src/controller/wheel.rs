//! The action wheel: opened by a knob gesture set to it, turned through its slices, the centre between the last and
//! the first; letting go of the knob, or a press, does the slice turned to (docs/project.md, section 3).

use super::{AMBER, Controller, action_colour};
use crate::display::screens::Screen;
use crate::knob::{PressAction, TurnAction};
use crate::overlay::{Slice, SliceIcon, Wheel};
use crate::soundboard::Action;
use crate::voicemod::VoicemodStatus;
use crate::wheel::WheelSettings;
use std::time::{Duration, Instant};

/// How long a wheel opened by a press or a plain turn stays after its last turn
pub(super) const WHEEL_TIME: Duration = Duration::from_secs(8);
/// How long the settings' preview shows the wheel
const PREVIEW_TIME: Duration = Duration::from_secs(3);
/// A page's slice: the window's amber
const PAGE_COLOUR: [u8; 3] = [255, 176, 32];

pub(super) struct WheelOpen {
    /// The slice turned to; None at the centre, where nothing happens
    selected: Option<usize>,
    /// Opened by turning the knob held: letting it go picks
    held: bool,
    /// When it closes by itself, unless held
    until: Option<Instant>,
    /// Shown from the settings, to see how it looks: the knob's gestures do not reach it
    preview: bool,
}

impl Controller {
    pub fn set_wheel(&mut self, wheel: WheelSettings) {
        let count = wheel.slices.len();
        self.wheel_settings = wheel;
        if let Some(open) = &mut self.wheel {
            open.selected = open.selected.filter(|index| *index < count);
        }
    }

    /// A turn the wheel takes: it opens on the first slice that way, or moves on
    pub(super) fn turn_wheel(&mut self, step: i8, held: bool, now: Instant) {
        if self.voicemod != VoicemodStatus::Connected {
            return;
        }
        let count = self.wheel_settings.slices.len();
        match &mut self.wheel {
            Some(open) => {
                open.selected = stepped(open.selected, step, count);
                if !open.held {
                    open.until = Some(now + WHEEL_TIME);
                }
            }
            None => {
                let first = if step > 0 { 0 } else { count.saturating_sub(1) };
                self.open_wheel(Some(first), held, now);
            }
        }
    }

    /// Opens the wheel at the centre, for the turns to pick and a press to do it
    pub(super) fn open_wheel_waiting(&mut self, now: Instant) {
        if self.voicemod == VoicemodStatus::Connected {
            self.open_wheel(None, false, now);
        }
    }

    fn open_wheel(&mut self, selected: Option<usize>, held: bool, now: Instant) {
        self.menu = None;
        self.row = None;
        self.level = None;
        self.peek = None;
        self.wheel = Some(WheelOpen {
            selected,
            held,
            until: (!held).then(|| now + WHEEL_TIME),
            preview: false,
        });
    }

    /// Shows the wheel on its first slice for a moment, to see how it looks
    pub fn preview_wheel(&mut self, now: Instant) {
        if self.voicemod == VoicemodStatus::Connected && !self.wheel_held() {
            self.open_wheel(Some(0), false, now);
            if let Some(open) = &mut self.wheel {
                open.until = Some(now + PREVIEW_TIME);
                open.preview = true;
            }
        }
    }

    /// Does the slice turned to, if any, and closes the wheel
    pub(super) fn pick_wheel(&mut self, now: Instant) {
        let Some(open) = self.wheel.take() else {
            return;
        };
        let slice = open
            .selected
            .and_then(|index| self.wheel_settings.slices.get(index))
            .cloned();
        if let Some(action) = slice {
            self.press_action(action, now);
        }
    }

    /// The wheel takes the knob's turns and presses while it waits for a press to pick
    pub(super) fn wheel_waits(&self) -> bool {
        self.wheel.as_ref().is_some_and(|open| !open.held && !open.preview)
    }

    /// A gesture goes to the knob as set, not to the wheel the settings show
    pub(super) fn drop_wheel_preview(&mut self) {
        if self.wheel.as_ref().is_some_and(|open| open.preview) {
            self.wheel = None;
        }
    }

    pub(super) fn wheel_held(&self) -> bool {
        self.wheel.as_ref().is_some_and(|open| open.held)
    }

    pub(super) fn close_wheel(&mut self) {
        self.wheel = None;
    }

    pub(super) fn update_wheel(&mut self, now: Instant) {
        self.wheel = self
            .wheel
            .take()
            .filter(|open| open.until.is_none_or(|until| until > now));
    }

    pub(super) fn wheel_until(&self) -> Option<Instant> {
        self.wheel.as_ref().and_then(|open| open.until)
    }

    /// The wheel for the PC screen, while open
    pub(super) fn wheel_content(&self) -> Option<Wheel> {
        let open = self.wheel.as_ref()?;
        let texts = &self.texts.wheel;
        let slices: Vec<Slice> = self
            .wheel_settings
            .slices
            .iter()
            .map(|action| self.slice(action))
            .collect();
        let centre = match open.selected.and_then(|index| slices.get(index)) {
            Some(slice) => slice.name.clone(),
            None => texts.cancel.to_string(),
        };
        // The preview says how the gesture set picks
        let held = open.held || (open.preview && self.knob_settings.hold_turn == TurnAction::Wheel);
        let hint = if held { texts.let_go } else { texts.press };
        Some(Wheel {
            slices,
            selected: open.selected,
            centre,
            hint: hint.to_string(),
        })
    }

    /// The wheel on Drezimo's display, while open
    pub(super) fn wheel_screen(&self) -> Option<Screen> {
        let open = self.wheel.as_ref()?;
        let texts = self.texts;
        let count = self.wheel_settings.slices.len();
        let chosen = open
            .selected
            .and_then(|index| Some((index, self.wheel_settings.slices.get(index)?)));
        let (name, kind, counter) = match chosen {
            Some((index, action)) => (
                self.slice(action).name,
                self.slice_kind(action).to_string(),
                format!("{}/{count}", index + 1),
            ),
            None => {
                let closes = if open.held {
                    texts.wheel.let_go_closes
                } else {
                    texts.wheel.press_closes
                };
                (texts.wheel.cancel.to_string(), closes.to_string(), "–".to_string())
            }
        };
        Some(Screen::Wheel {
            heading: texts.wheel.heading.to_string(),
            counter,
            name,
            kind,
            slices: count,
            selected: open.selected,
        })
    }

    fn slice(&self, action: &PressAction) -> Slice {
        let texts = self.texts;
        let acting = |action: Action, icon| Slice {
            name: texts.action_name(action).to_string(),
            icon,
            colour: action_colour(action),
        };
        match action {
            PressAction::Page { page } => Slice {
                name: self
                    .pages
                    .iter()
                    .find(|shown| shown.soundboard_id == *page)
                    .map_or(texts.wheel.missing, |shown| self.page_name(shown))
                    .to_string(),
                icon: SliceIcon::Page,
                colour: PAGE_COLOUR,
            },
            PressAction::Macro { macro_id } => {
                let item = self.macros.iter().find(|item| item.id == *macro_id);
                Slice {
                    name: item.map_or(texts.wheel.missing, |item| item.name.as_str()).to_string(),
                    icon: SliceIcon::Macro,
                    colour: item.and_then(|item| item.colour).unwrap_or(AMBER),
                }
            }
            PressAction::Menu => Slice {
                name: texts.wheel.menu.to_string(),
                icon: SliceIcon::Menu,
                colour: PAGE_COLOUR,
            },
            PressAction::StopSounds => acting(Action::StopSounds, SliceIcon::StopSounds),
            PressAction::MuteMic => acting(Action::MuteMic, SliceIcon::MuteMic),
            PressAction::MuteForMe => acting(Action::MuteForMe, SliceIcon::MuteForMe),
            PressAction::VoiceChanger => acting(Action::VoiceChanger, SliceIcon::VoiceChanger),
            PressAction::HearMyself => acting(Action::HearMyself, SliceIcon::HearMyself),
            PressAction::Background => acting(Action::Background, SliceIcon::Background),
            PressAction::RandomVoice => acting(Action::RandomVoice, SliceIcon::RandomVoice),
            PressAction::Overlay => acting(Action::Overlay, SliceIcon::Overlay),
            PressAction::ResetVolumes | PressAction::Wheel | PressAction::Nothing => {
                acting(Action::ResetVolumes, SliceIcon::ResetVolumes)
            }
        }
    }

    fn slice_kind(&self, action: &PressAction) -> &'static str {
        let texts = self.texts;
        match action {
            PressAction::Page { .. } | PressAction::Menu => texts.wheel.page,
            PressAction::Macro { .. } => texts.macro_info,
            _ => texts.action,
        }
    }
}

/// The place `step` away: the centre, then each slice, round again
fn stepped(selected: Option<usize>, step: i8, count: usize) -> Option<usize> {
    let at = selected.map_or(0, |index| index + 1) as isize;
    let next = (at + isize::from(step)).rem_euclid(count as isize + 1);
    (next > 0).then(|| next as usize - 1)
}

#[cfg(test)]
mod tests {
    use super::stepped;

    #[test]
    fn the_centre_sits_between_the_last_slice_and_the_first() {
        assert_eq!(stepped(None, 1, 4), Some(0));
        assert_eq!(stepped(Some(0), -1, 4), None);
        assert_eq!(stepped(None, -1, 4), Some(3));
        assert_eq!(stepped(Some(3), 1, 4), None);
        assert_eq!(stepped(Some(1), 1, 4), Some(2));
    }
}
