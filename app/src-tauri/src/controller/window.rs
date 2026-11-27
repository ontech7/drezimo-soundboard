//! The board as the window's Status page shows it: the current page's pads and the function keys, with their LEDs, so
//! it can be played from the window too.

use super::{Controller, PADS};
use crate::pictures;
use crate::protocol::{BUTTON_COUNT, BoardMessage, Led};
use crate::soundboard::{Action, Pad};
use crate::voicemod::VoicemodStatus;
use serde::Serialize;
use std::time::Instant;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardView {
    /// The pages' titles, as the soundboard menu lists them
    pub pages: Vec<String>,
    /// Each page's soundboard id, or the id of the user's own page it is
    pub ids: Vec<String>,
    /// How many of the pages, from the first, are the user's own
    pub own: usize,
    pub page: usize,
    pub pads: Vec<PadView>,
    /// The function keys' LEDs, from the left
    pub keys: Vec<Led>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PadView {
    /// Empty for a pad without a sound or voice
    pub name: String,
    /// A voice, rather than a sound
    pub voice: bool,
    /// Its picture's path on the pad-image scheme (src/pictures.rs)
    pub picture: Option<String>,
    /// What an action pad does, for its icon
    pub action: Option<Action>,
    #[serde(rename = "macro")]
    pub is_macro: bool,
    pub led: Led,
    /// Outside the licence
    pub locked: bool,
}

impl Controller {
    /// None while there is nothing to play: Voicemod away, or no soundboards.
    pub fn board_view(&self, now: Instant) -> Option<BoardView> {
        if self.voicemod != VoicemodStatus::Connected || self.pages.is_empty() {
            return None;
        }
        let pads = (0..PADS as u8)
            .map(|pad| {
                let content = self.pad_at(pad);
                PadView {
                    name: content.map(|content| content.name().to_string()).unwrap_or_default(),
                    voice: matches!(content, Some(Pad::Voice(_))),
                    picture: content.and_then(pictures::path),
                    action: match content {
                        Some(Pad::Action { action, .. }) => Some(*action),
                        _ => None,
                    },
                    is_macro: matches!(content, Some(Pad::Macro { .. })),
                    led: self.led(usize::from(pad), now),
                    locked: content.is_some_and(|content| !content.enabled()),
                }
            })
            .collect();
        Some(BoardView {
            pages: self.pages.iter().map(|page| self.page_title(page)).collect(),
            ids: self.pages.iter().map(|page| page.soundboard_id.clone()).collect(),
            own: self.custom.len(),
            page: self.page,
            pads,
            keys: (0..usize::from(BUTTON_COUNT))
                .map(|key| self.led(PADS + key, now))
                .collect(),
        })
    }

    /// A pad pressed or released in the window, as on the board. A pad already held on the board stays the board's.
    pub fn window_pad(&mut self, pad: u8, pressed: bool, now: Instant) {
        if pressed {
            if !self.held.contains_key(&pad) {
                self.window_held.insert(pad);
                self.pad_pressed(pad, now);
            }
        } else if self.window_held.remove(&pad) {
            self.pad_released(pad, now);
        }
    }

    /// The knob turned in the window, as on the board: gestures and all.
    pub fn window_knob_turned(&mut self, step: i8, now: Instant) {
        self.knob_turned(step, now);
    }

    pub fn window_knob_pressed(&mut self, pressed: bool, now: Instant) {
        self.knob_pressed(pressed, now);
    }

    /// A function key clicked in the window, as on the board.
    pub fn window_key(&mut self, key: u8, now: Instant) {
        self.board_message(
            BoardMessage::Button {
                index: key,
                pressed: true,
            },
            now,
        );
    }

    /// A page picked in the window: no overlay, since the window shows the page, and no knob menu left open on another.
    pub fn go_to_page(&mut self, page: usize) {
        if page < self.pages.len() {
            self.change_page(page);
            self.menu = None;
        }
    }
}
