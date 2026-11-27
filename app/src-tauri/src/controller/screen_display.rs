//! When Drezimo's display shows on the PC screen: never, always, or with the actions the Overlay page picks and a moment
//! after them (docs/project.md, section 8).

use super::{Controller, Output};
use crate::settings::{ScreenDisplaySettings, ShowDisplay};
use std::time::{Duration, Instant};

/// It stays this long after its action, to see how it ended
const LINGER: Duration = Duration::from_millis(1500);
/// After a page change, as the overlay shows for a while
const PAGE_TIME: Duration = Duration::from_millis(2500);
/// The Overlay page's preview
const PREVIEW_TIME: Duration = Duration::from_secs(3);

impl Controller {
    pub fn set_screen_display(&mut self, settings: ScreenDisplaySettings) {
        self.screen_display = settings;
    }

    /// Shows it for a moment, whatever it is set to, to see where it sits
    pub fn preview_screen_display(&mut self, now: Instant) {
        self.display_preview_until = Some(now + PREVIEW_TIME);
    }

    /// One of the actions it shows with is under way
    fn display_wanted(&self) -> bool {
        let settings = &self.screen_display;
        let knob = self.knob.is_down() || self.row.is_some() || self.peek.is_some() || self.wheel.is_some();
        (settings.on_menu && self.menu.is_some())
            || (settings.on_volumes && self.level.is_some())
            || (settings.on_knob && knob)
    }

    pub(super) fn update_screen_display(&mut self, now: Instant) {
        let on_actions = self.screen_display.when == ShowDisplay::OnActions;
        // By the page itself: the pages coming back after Voicemod restarts, or one added before it, are no change
        let page = self.page_id(self.page);
        if page != self.display_page {
            if self.display_page.is_some() && page.is_some() && on_actions && self.screen_display.on_page {
                self.display_until = self.display_until.max(Some(now + PAGE_TIME));
            }
            if page.is_some() {
                self.display_page = page;
            }
        }
        let wanted = on_actions && self.display_wanted();
        // It stays a moment after the action ends, from then
        if self.display_wanted_before && !wanted && on_actions {
            self.display_until = self.display_until.max(Some(now + LINGER));
        }
        self.display_wanted_before = wanted;
        if !on_actions {
            self.display_until = None;
        }
        self.display_until = self.display_until.filter(|until| *until > now);
        self.display_preview_until = self.display_preview_until.filter(|until| *until > now);
        let shown = match self.screen_display.when {
            ShowDisplay::Never => false,
            ShowDisplay::Always => true,
            ShowDisplay::OnActions => wanted || self.display_until.is_some(),
        } || self.display_preview_until.is_some();
        if shown != self.display_shown {
            self.display_shown = shown;
            self.outputs.push(Output::ScreenDisplay(shown));
        }
    }

    pub(super) fn screen_display_wake(&self) -> Option<Instant> {
        [self.display_until, self.display_preview_until]
            .into_iter()
            .flatten()
            .min()
    }
}
