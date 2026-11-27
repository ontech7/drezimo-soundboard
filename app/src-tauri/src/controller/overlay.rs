//! What the PC overlay shows: the current page with each pad's sound and LED, while the board is in use.

use super::{Controller, PADS};
use crate::overlay::{Cell, Content, Overlay};
use crate::voicemod::VoicemodStatus;
use std::time::{Duration, Instant};

/// How long the overlay stays after the page changed or the knob was let go, unless set otherwise
pub(super) const OVERLAY_TIME: Duration = Duration::from_secs(4);

impl Controller {
    /// What the overlay's window shows: the action wheel while open, else the page while shown.
    pub(super) fn overlay_content(&self, now: Instant) -> Option<Content> {
        if self.voicemod == VoicemodStatus::Connected
            && let Some(wheel) = self.wheel_content()
        {
            return Some(Content::Wheel(wheel));
        }
        self.overlay(now).map(Content::Page)
    }

    /// The overlay for `now`, or None while it is hidden.
    pub(super) fn overlay(&self, now: Instant) -> Option<Overlay> {
        let page = self.pages.get(self.page).filter(|_| self.overlay_shown(now))?;
        let cells = (0..PADS as u8)
            .map(|pad| {
                let content = self.pad_at(pad);
                Cell {
                    name: content.map(|content| content.name().to_string()).unwrap_or_default(),
                    led: self.pad_led(pad, now),
                    locked: content.is_some_and(|content| !content.enabled()),
                }
            })
            .collect();
        Some(Overlay {
            title: self.page_title(page),
            counter: self.counter(),
            cells,
        })
    }

    /// Whether the overlay is on screen at `now`
    pub(super) fn overlay_shown(&self, now: Instant) -> bool {
        let always = self.overlay_always && !self.overlay_hidden;
        let asked =
            always || self.overlay_until.is_some_and(|until| until > now) || self.knob.is_down() || self.menu.is_some();
        let wanted = asked && self.overlay_time.is_some();
        let usable = self.board_connected && self.voicemod == VoicemodStatus::Connected;
        wanted && usable && !self.pages.is_empty()
    }

    pub(super) fn show_overlay(&mut self, now: Instant) {
        self.overlay_until = self.overlay_time.map(|time| now + time);
    }

    /// How long the overlay stays, or None to never show it.
    pub fn set_overlay_time(&mut self, time: Option<Duration>) {
        self.overlay_time = time;
    }

    /// Shown all the time, until a knob gesture hides it: a new choice shows it again.
    pub fn set_overlay_always(&mut self, always: bool) {
        if always != self.overlay_always {
            self.overlay_always = always;
            self.overlay_hidden = false;
        }
    }

    /// The knob's Overlay gesture: hides or shows the overlay that shows always, else shows it for its time.
    pub(super) fn overlay_gesture(&mut self, now: Instant) {
        if self.overlay_always {
            self.overlay_hidden = !self.overlay_hidden;
            if self.overlay_hidden {
                self.overlay_until = None;
            }
        } else {
            self.show_overlay(now);
        }
    }

    /// Hides the overlay a connection would show, for a board that was only lent, as to a board test.
    pub fn hide_overlay(&mut self) {
        self.overlay_until = None;
    }

    /// Shows the overlay now, as a page change would, to see where it sits.
    pub fn preview_overlay(&mut self, now: Instant) {
        self.show_overlay(now);
    }
}
