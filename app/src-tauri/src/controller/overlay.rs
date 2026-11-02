//! What the PC overlay shows: the current page with each pad's sound and LED, while the board is in use.

use super::{Controller, PADS};
use crate::overlay::{Cell, Overlay};
use crate::voicemod::VoicemodStatus;
use std::time::{Duration, Instant};

/// How long the overlay stays after the page changed or the knob was let go, unless set otherwise
pub(super) const OVERLAY_TIME: Duration = Duration::from_secs(4);

impl Controller {
    /// The overlay for `now`, or None while it is hidden.
    pub(super) fn overlay(&self, now: Instant) -> Option<Overlay> {
        let asked = self.overlay_until.is_some_and(|until| until > now) || self.knob.pressed || self.menu.is_some();
        let wanted = asked && self.overlay_time.is_some();
        let usable = self.board_connected && self.voicemod == VoicemodStatus::Connected;
        let page = self.pages.get(self.page).filter(|_| wanted && usable)?;
        let cells = (0..PADS as u8)
            .map(|pad| {
                let sound = self.sound_at(pad);
                Cell {
                    name: sound.map(|sound| sound.name.clone()).unwrap_or_default(),
                    led: self.pad_led(pad, now),
                    locked: sound.is_some_and(|sound| !sound.enabled),
                }
            })
            .collect();
        Some(Overlay {
            title: self.page_title(page),
            counter: self.counter(),
            cells,
        })
    }

    pub(super) fn show_overlay(&mut self, now: Instant) {
        self.overlay_until = self.overlay_time.map(|time| now + time);
    }

    /// How long the overlay stays, or None to never show it.
    pub fn set_overlay_time(&mut self, time: Option<Duration>) {
        self.overlay_time = time;
    }

    /// Shows the overlay now, as a page change would, to see where it sits.
    pub fn preview_overlay(&mut self, now: Instant) {
        self.show_overlay(now);
    }
}
