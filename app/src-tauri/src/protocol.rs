//! The messages between the board and the app, as described in docs/project.md, section 9.

use serde::{Deserialize, Serialize};

pub const PAD_COUNT: u8 = 36;
/// The pads, row by row from the top left
pub const COLUMNS: usize = 6;
pub const ROWS: usize = PAD_COUNT as usize / COLUMNS;
pub const BUTTON_COUNT: u8 = 6;
/// The pads' LEDs come first in the chain, then the function keys'
pub const LED_COUNT: usize = (PAD_COUNT + BUTTON_COUNT) as usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Led {
    pub color: [u8; 3],
    pub effect: Effect,
}

/// Animated by the board, so the effects stay smooth whatever the USB timing
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Effect {
    Solid,
    Breathe,
    Blink,
}

/// A display picture, from the hex of its bytes.
pub fn display_message(hex: &str) -> String {
    format!("DISPLAY,{hex}")
}

impl Led {
    pub const OFF: Self = Self::solid([0, 0, 0]);

    pub const fn solid(color: [u8; 3]) -> Self {
        Self {
            color,
            effect: Effect::Solid,
        }
    }

    pub fn message(&self, index: usize) -> String {
        let [r, g, b] = self.color;
        match self.effect {
            Effect::Solid => format!("LED,{index},{r},{g},{b}"),
            Effect::Breathe => format!("LED,{index},{r},{g},{b},BREATHE"),
            Effect::Blink => format!("LED,{index},{r},{g},{b},BLINK"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum BoardMessage {
    Hello {
        firmware: String,
    },
    Pad {
        index: u8,
        pressed: bool,
    },
    Button {
        index: u8,
        pressed: bool,
    },
    Encoder {
        step: i8,
    },
    EncoderButton {
        pressed: bool,
    },
    /// The reply to SETTINGS
    Settings(DisplaySettings),
    Ok,
    Error {
        reason: String,
    },
}

/// The display's settings, which the board keeps in its flash
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplaySettings {
    /// In percent
    pub brightness: u8,
    /// Minutes without any input before the display dims, and before it turns off: 0 is never
    pub dim_after: u16,
    pub off_after: u16,
    /// Minutes between two pixel shifts: 0 is off
    pub shift_every: u16,
}

impl DisplaySettings {
    /// The SET messages that turn `self` into `wanted`.
    pub fn changes(&self, wanted: &Self) -> Vec<String> {
        let fields = [
            ("OLED_BRIGHTNESS", self.brightness.into(), wanted.brightness.into()),
            ("OLED_DIM_AFTER", self.dim_after, wanted.dim_after),
            ("OLED_OFF_AFTER", self.off_after, wanted.off_after),
            ("OLED_SHIFT", self.shift_every, wanted.shift_every),
        ];
        fields
            .into_iter()
            .filter(|(_, now, then)| now != then)
            .map(|(name, _, value)| format!("SET,{name},{value}"))
            .collect()
    }
}

impl BoardMessage {
    pub fn parse(line: &str) -> Option<Self> {
        let line = line.trim();
        // A text field is always the last one and may contain commas
        if let Some(firmware) = line.strip_prefix("DREZIMO,") {
            return Some(Self::Hello {
                firmware: firmware.to_string(),
            });
        }
        if let Some(reason) = line.strip_prefix("ERR,") {
            return Some(Self::Error {
                reason: reason.to_string(),
            });
        }

        let fields: Vec<&str> = line.split(',').collect();
        let message = match fields.as_slice() {
            ["PAD", index, state] => Self::Pad {
                index: parse_index(index, PAD_COUNT)?,
                pressed: parse_pressed(state)?,
            },
            ["BTN", index, state] => Self::Button {
                index: parse_index(index, BUTTON_COUNT)?,
                pressed: parse_pressed(state)?,
            },
            ["ENC", "+1"] => Self::Encoder { step: 1 },
            ["ENC", "-1"] => Self::Encoder { step: -1 },
            ["ENCBTN", state] => Self::EncoderButton {
                pressed: parse_pressed(state)?,
            },
            ["SETTINGS", brightness, dim_after, off_after, shift_every] => Self::Settings(DisplaySettings {
                brightness: brightness.parse().ok()?,
                dim_after: dim_after.parse().ok()?,
                off_after: off_after.parse().ok()?,
                shift_every: shift_every.parse().ok()?,
            }),
            ["OK"] => Self::Ok,
            _ => return None,
        };
        Some(message)
    }
}

fn parse_index(text: &str, count: u8) -> Option<u8> {
    text.parse().ok().filter(|&index| index < count)
}

fn parse_pressed(text: &str) -> Option<bool> {
    match text {
        "1" => Some(true),
        "0" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_board_message() {
        let cases = [
            (
                "DREZIMO,0.2.0",
                BoardMessage::Hello {
                    firmware: "0.2.0".into(),
                },
            ),
            (
                "PAD,35,1",
                BoardMessage::Pad {
                    index: 35,
                    pressed: true,
                },
            ),
            (
                "BTN,0,0",
                BoardMessage::Button {
                    index: 0,
                    pressed: false,
                },
            ),
            ("ENC,+1", BoardMessage::Encoder { step: 1 }),
            ("ENC,-1", BoardMessage::Encoder { step: -1 }),
            ("ENCBTN,1", BoardMessage::EncoderButton { pressed: true }),
            (
                "SETTINGS,50,1,10,3",
                BoardMessage::Settings(DisplaySettings {
                    brightness: 50,
                    dim_after: 1,
                    off_after: 10,
                    shift_every: 3,
                }),
            ),
            ("OK", BoardMessage::Ok),
            (
                "ERR,unknown message",
                BoardMessage::Error {
                    reason: "unknown message".into(),
                },
            ),
        ];
        for (line, expected) in cases {
            assert_eq!(BoardMessage::parse(line), Some(expected), "{line}");
        }
    }

    #[test]
    fn sets_only_the_display_settings_that_changed() {
        let saved = DisplaySettings {
            brightness: 50,
            dim_after: 1,
            off_after: 10,
            shift_every: 3,
        };
        let wanted = DisplaySettings {
            brightness: 80,
            off_after: 0,
            ..saved
        };
        assert_eq!(
            saved.changes(&wanted),
            ["SET,OLED_BRIGHTNESS,80", "SET,OLED_OFF_AFTER,0"]
        );
        assert!(saved.changes(&saved).is_empty());
    }

    #[test]
    fn keeps_commas_in_text_fields() {
        assert_eq!(
            BoardMessage::parse("DREZIMO,0.3.0,beta"),
            Some(BoardMessage::Hello {
                firmware: "0.3.0,beta".into()
            })
        );
        assert_eq!(
            BoardMessage::parse("ERR,bad value, expected 0-100"),
            Some(BoardMessage::Error {
                reason: "bad value, expected 0-100".into()
            })
        );
    }

    #[test]
    fn accepts_line_endings() {
        assert_eq!(
            BoardMessage::parse("PAD,3,0\r\n"),
            Some(BoardMessage::Pad {
                index: 3,
                pressed: false
            })
        );
    }

    #[test]
    fn writes_led_messages() {
        assert_eq!(Led::solid([47, 107, 255]).message(0), "LED,0,47,107,255");
        let blinking = Led {
            color: [255, 42, 42],
            effect: Effect::Blink,
        };
        assert_eq!(blinking.message(41), "LED,41,255,42,42,BLINK");
    }

    #[test]
    fn rejects_malformed_lines() {
        for line in [
            "",
            "PAD,36,1",
            "PAD,1",
            "PAD,1,2",
            "PAD,-1,1",
            "BTN,6,1",
            "ENC,+2",
            "ENC,1",
            "PAD,1,1,1",
            "HELLO",
        ] {
            assert_eq!(BoardMessage::parse(line), None, "{line:?}");
        }
    }
}
