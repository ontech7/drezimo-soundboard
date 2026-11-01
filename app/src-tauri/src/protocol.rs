//! Messages sent by the board, as described in docs/project.md, section 9.

use serde::Serialize;

pub const PAD_COUNT: u8 = 36;
pub const BUTTON_COUNT: u8 = 6;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum BoardMessage {
    Hello { firmware: String },
    Pad { index: u8, pressed: bool },
    Button { index: u8, pressed: bool },
    Encoder { step: i8 },
    EncoderButton { pressed: bool },
    Ok,
    Error { reason: String },
}

impl BoardMessage {
    pub fn parse(line: &str) -> Option<Self> {
        let fields: Vec<&str> = line.trim().split(',').collect();
        let message = match fields.as_slice() {
            ["DREZIMO", firmware] => Self::Hello { firmware: firmware.to_string() },
            ["PAD", index, state] => Self::Pad { index: parse_index(index, PAD_COUNT)?, pressed: parse_pressed(state)? },
            ["BTN", index, state] => Self::Button { index: parse_index(index, BUTTON_COUNT)?, pressed: parse_pressed(state)? },
            ["ENC", "+1"] => Self::Encoder { step: 1 },
            ["ENC", "-1"] => Self::Encoder { step: -1 },
            ["ENCBTN", state] => Self::EncoderButton { pressed: parse_pressed(state)? },
            ["OK"] => Self::Ok,
            ["ERR", reason] => Self::Error { reason: reason.to_string() },
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
            ("DREZIMO,0.2.0", BoardMessage::Hello { firmware: "0.2.0".into() }),
            ("PAD,35,1", BoardMessage::Pad { index: 35, pressed: true }),
            ("BTN,0,0", BoardMessage::Button { index: 0, pressed: false }),
            ("ENC,+1", BoardMessage::Encoder { step: 1 }),
            ("ENC,-1", BoardMessage::Encoder { step: -1 }),
            ("ENCBTN,1", BoardMessage::EncoderButton { pressed: true }),
            ("OK", BoardMessage::Ok),
            ("ERR,unknown message", BoardMessage::Error { reason: "unknown message".into() }),
        ];
        for (line, expected) in cases {
            assert_eq!(BoardMessage::parse(line), Some(expected), "{line}");
        }
    }

    #[test]
    fn accepts_line_endings() {
        assert_eq!(BoardMessage::parse("PAD,3,0\r\n"), Some(BoardMessage::Pad { index: 3, pressed: false }));
    }

    #[test]
    fn rejects_malformed_lines() {
        for line in ["", "PAD,36,1", "PAD,1", "PAD,1,2", "PAD,-1,1", "BTN,6,1", "ENC,+2", "ENC,1", "PAD,1,1,1", "HELLO"] {
            assert_eq!(BoardMessage::parse(line), None, "{line:?}");
        }
    }
}
