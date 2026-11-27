//! The action wheel: shortcuts picked by turning the knob, shown on the PC screen around Drezimo's logo
//! (docs/project.md, section 3).

use crate::knob::PressAction;
use crate::settings::Anchor;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

pub const MIN_SLICES: usize = 2;
pub const MAX_SLICES: usize = 8;
/// The longest page or macro id a slice keeps, as a gesture's
const MAX_TARGET: usize = 64;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WheelSettings {
    /// Clockwise from the top
    #[serde(deserialize_with = "readable_slices")]
    pub slices: Vec<PressAction>,
    /// Its own place, size and opacity; otherwise the overlay's
    pub own_look: bool,
    pub anchor: Anchor,
    /// In percent of the full size
    pub size: u8,
    /// In percent
    pub opacity: u8,
}

impl Default for WheelSettings {
    fn default() -> Self {
        Self {
            slices: vec![
                PressAction::StopSounds,
                PressAction::MuteMic,
                PressAction::RandomVoice,
                PressAction::VoiceChanger,
                PressAction::HearMyself,
                PressAction::Overlay,
            ],
            own_look: false,
            anchor: Anchor::Center,
            size: 100,
            opacity: 100,
        }
    }
}

impl WheelSettings {
    /// Within what the window offers: a slice can neither open the wheel again nor do nothing
    pub fn checked(self) -> Self {
        let mut slices: Vec<PressAction> = self
            .slices
            .into_iter()
            .filter(|slice| match slice {
                PressAction::Wheel | PressAction::Nothing => false,
                PressAction::Macro { macro_id: id } | PressAction::Page { page: id } => id.len() <= MAX_TARGET,
                _ => true,
            })
            .take(MAX_SLICES)
            .collect();
        if slices.len() < MIN_SLICES {
            slices = Self::default().slices;
        }
        Self {
            slices,
            size: self.size.clamp(60, 130),
            opacity: self.opacity.clamp(40, 100),
            ..self
        }
    }
}

/// A slice that cannot be read, from another version, is left out rather than losing the others
fn readable_slices<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<PressAction>, D::Error> {
    let value = Value::deserialize(deserializer)?;
    let Value::Array(items) = value else {
        return Ok(WheelSettings::default().slices);
    };
    Ok(items
        .into_iter()
        .filter_map(|item| PressAction::deserialize(item).ok())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slice_never_opens_the_wheel_again() {
        let wheel = WheelSettings {
            slices: vec![
                PressAction::Wheel,
                PressAction::MuteMic,
                PressAction::Nothing,
                PressAction::Page { page: "p".repeat(65) },
                PressAction::Page { page: "mine".into() },
            ],
            ..WheelSettings::default()
        }
        .checked();
        assert_eq!(
            wheel.slices,
            [PressAction::MuteMic, PressAction::Page { page: "mine".into() }]
        );
    }

    #[test]
    fn keeps_eight_slices_at_most_and_two_at_least() {
        let many = WheelSettings {
            slices: vec![PressAction::MuteMic; 12],
            ..WheelSettings::default()
        }
        .checked();
        assert_eq!(many.slices.len(), MAX_SLICES);
        let one = WheelSettings {
            slices: vec![PressAction::MuteMic],
            ..WheelSettings::default()
        }
        .checked();
        assert_eq!(one.slices, WheelSettings::default().slices);
    }

    #[test]
    fn keeps_size_and_opacity_within_the_window_s_ranges() {
        let wheel = WheelSettings {
            size: 200,
            opacity: 0,
            ..WheelSettings::default()
        }
        .checked();
        assert_eq!((wheel.size, wheel.opacity), (130, 40));
    }

    #[test]
    fn an_unreadable_slice_is_left_out_alone() {
        let wheel: WheelSettings = serde_json::from_str(
            r#"{"slices": [{"action": "muteMic"}, {"action": "dance"}, {"action": "page", "page": "mine"}], "ownLook": true}"#,
        )
        .unwrap();
        assert_eq!(
            wheel.slices,
            [PressAction::MuteMic, PressAction::Page { page: "mine".into() }]
        );
        assert!(wheel.own_look);
        assert_eq!(wheel.anchor, Anchor::Center);
    }

    #[test]
    fn round_trips() {
        let wheel = WheelSettings {
            slices: vec![
                PressAction::Macro {
                    macro_id: "intro".into(),
                },
                PressAction::StopSounds,
            ],
            own_look: true,
            anchor: Anchor::Top,
            size: 80,
            opacity: 90,
        };
        let json = serde_json::to_value(&wheel).unwrap();
        assert_eq!(
            json["slices"][0],
            serde_json::json!({"action": "macro", "macroId": "intro"})
        );
        assert_eq!(serde_json::from_value::<WheelSettings>(json).unwrap(), wheel);
    }
}
