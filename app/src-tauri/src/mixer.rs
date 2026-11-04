//! Voicemod's volumes as the knob changes them: its mixer's channels and the volume of a sound playing.

use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Channel {
    SoundboardForOthers,
    SoundboardForMe,
    Microphone,
}

impl Channel {
    pub const ALL: [Channel; 3] = [
        Channel::SoundboardForOthers,
        Channel::SoundboardForMe,
        Channel::Microphone,
    ];

    /// Voicemod's name for it in its bridge's messages
    pub fn control_type(self) -> &'static str {
        match self {
            Channel::SoundboardForOthers => "streamingSoundboardPreMix",
            Channel::SoundboardForMe => "monitorSoundboardPreMix",
            Channel::Microphone => "input",
        }
    }

    pub fn from_control_type(text: &str) -> Option<Channel> {
        Channel::ALL.into_iter().find(|channel| channel.control_type() == text)
    }

    /// The top of Voicemod's slider
    pub fn max_percent(self) -> u16 {
        match self {
            Channel::Microphone => 200,
            _ => 100,
        }
    }
}

/// The knob's step, in percent of the slider
pub const STEP: u16 = 5;

/// The gain of a mixer slider at that percent, by Voicemod's own curve: squared up to 100 %, linear above.
pub fn gain(percent: u16) -> f32 {
    let share = f32::from(percent) / 100.0;
    if percent <= 100 { share * share } else { share }
}

pub fn percent(gain: f32) -> u16 {
    let gain = gain.max(0.0);
    let share = if gain <= 1.0 { gain.sqrt() } else { gain };
    (share * 100.0).round() as u16
}

/// What puts the knob's changes back
#[derive(Debug, Default, PartialEq)]
pub struct Reset {
    pub channels: Vec<(Channel, f32)>,
    pub sounds: Vec<(String, f32)>,
}

#[derive(Default)]
pub struct Mixer {
    /// Voicemod's own value of each channel: read when the bridge connects, then moved by its own sliders
    voicemods: HashMap<Channel, f32>,
    /// Each channel now, the knob's changes included
    current: HashMap<Channel, f32>,
    /// Each sound's volume saved in Voicemod, 0 to 1
    saved: HashMap<String, f32>,
    /// The volume the knob gave a sound for the play under way
    playing: HashMap<String, f32>,
}

impl Mixer {
    /// The bridge connected again: every value comes again from Voicemod
    pub fn connected(&mut self) {
        *self = Self::default();
    }

    /// A channel's gain, as Voicemod tells it. `ours`: the knob's own change coming back, older than its latest step.
    pub fn voicemod_gain(&mut self, channel: Channel, gain: f32, ours: bool) {
        if ours {
            return;
        }
        self.voicemods.insert(channel, gain);
        self.current.insert(channel, gain);
    }

    pub fn sound_volumes(&mut self, volumes: HashMap<String, f32>) {
        self.saved = volumes;
    }

    /// The channel's percent after `steps` clicks, or None while its value is not known yet
    pub fn step_gain(&mut self, channel: Channel, steps: i8) -> Option<u16> {
        let now = percent(*self.current.get(&channel)?);
        let next = stepped(now, steps, channel.max_percent());
        self.current.insert(channel, gain(next));
        Some(next)
    }

    /// The sound's percent after `steps` clicks, from its saved volume at the start of each play
    pub fn step_sound(&mut self, sound: &str, steps: i8) -> Option<u16> {
        let volume = self.playing.get(sound).or_else(|| self.saved.get(sound))?;
        let next = stepped((volume * 100.0).round() as u16, steps, 100);
        self.playing.insert(sound.to_string(), f32::from(next) / 100.0);
        Some(next)
    }

    /// The sounds that ended start from their saved volume at their next play
    pub fn keep_playing(&mut self, playing: impl Fn(&str) -> bool) {
        self.playing.retain(|sound, _| playing(sound));
    }

    /// Each channel away from Voicemod's own value, and each sound still playing at another volume than its saved one
    pub fn reset(&mut self) -> Reset {
        let mut channels = Vec::new();
        for channel in Channel::ALL {
            if let (Some(&own), Some(&now)) = (self.voicemods.get(&channel), self.current.get(&channel))
                && own != now
            {
                channels.push((channel, own));
                self.current.insert(channel, own);
            }
        }
        let mut sounds: Vec<_> = self
            .playing
            .drain()
            .filter_map(|(sound, _)| Some((sound.clone(), *self.saved.get(&sound)?)))
            .collect();
        sounds.sort_by(|a, b| a.0.cmp(&b.0));
        Reset { channels, sounds }
    }
}

fn stepped(percent: u16, steps: i8, max: u16) -> u16 {
    let next = i32::from(percent) + i32::from(steps) * i32::from(STEP);
    next.clamp(0, i32::from(max)) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mixer() -> Mixer {
        let mut mixer = Mixer::default();
        mixer.voicemod_gain(Channel::SoundboardForOthers, 1.0, false);
        mixer.voicemod_gain(Channel::SoundboardForMe, 0.25, false);
        mixer.voicemod_gain(Channel::Microphone, gain(195), false);
        mixer.sound_volumes(HashMap::from([("song".into(), 0.3), ("honk".into(), 1.0)]));
        mixer
    }

    #[test]
    fn the_curve_matches_voicemods_sliders() {
        assert_eq!(gain(50), 0.25);
        assert_eq!(gain(100), 1.0);
        assert_eq!(gain(150), 1.5);
        assert_eq!(gain(0), 0.0);
        assert_eq!(percent(0.3025), 55);
        assert_eq!(percent(0.304), 55);
        assert_eq!(percent(0.25), 50);
        assert_eq!(percent(1.5), 150);
        for value in 0..=200 {
            assert_eq!(percent(gain(value)), value);
        }
    }

    #[test]
    fn channels_have_voicemods_names() {
        for channel in Channel::ALL {
            assert_eq!(Channel::from_control_type(channel.control_type()), Some(channel));
        }
        assert_eq!(Channel::SoundboardForOthers.control_type(), "streamingSoundboardPreMix");
        assert_eq!(Channel::SoundboardForMe.control_type(), "monitorSoundboardPreMix");
        assert_eq!(Channel::Microphone.control_type(), "input");
        assert_eq!(Channel::from_control_type("streaming"), None);
        assert_eq!(Channel::Microphone.max_percent(), 200);
        assert_eq!(Channel::SoundboardForMe.max_percent(), 100);
    }

    #[test]
    fn steps_are_five_percent_within_the_slider() {
        let mut mixer = mixer();
        assert_eq!(mixer.step_gain(Channel::SoundboardForOthers, -1), Some(95));
        assert_eq!(mixer.step_gain(Channel::SoundboardForOthers, 3), Some(100));
        assert_eq!(mixer.step_gain(Channel::SoundboardForMe, -10), Some(0));
        assert_eq!(mixer.step_gain(Channel::SoundboardForMe, -1), Some(0));
        assert_eq!(mixer.step_gain(Channel::Microphone, 2), Some(200));
        assert_eq!(mixer.step_gain(Channel::Microphone, -1), Some(195));
        assert_eq!(mixer.step_sound("honk", 1), Some(100));
        assert_eq!(mixer.step_sound("song", -10), Some(0));
    }

    #[test]
    fn nothing_is_stepped_before_voicemod_tells_the_value() {
        let mut mixer = Mixer::default();
        assert_eq!(mixer.step_gain(Channel::SoundboardForOthers, 1), None);
        assert_eq!(mixer.step_sound("song", 1), None);
        mixer.voicemod_gain(Channel::SoundboardForOthers, 0.25, false);
        assert_eq!(mixer.step_gain(Channel::SoundboardForOthers, 1), Some(55));
        assert_eq!(mixer.step_gain(Channel::Microphone, 1), None);
    }

    #[test]
    fn voicemods_own_slider_moves_the_reset_target_but_ours_does_not() {
        let mut mixer = mixer();
        mixer.step_gain(Channel::SoundboardForOthers, -2);
        // Our own change, coming back from the bridge
        mixer.voicemod_gain(Channel::SoundboardForOthers, gain(90), true);
        let reset = mixer.reset();
        assert_eq!(reset.channels, [(Channel::SoundboardForOthers, 1.0)]);
        assert!(reset.sounds.is_empty());
        assert_eq!(mixer.reset(), Reset::default());

        mixer.step_gain(Channel::SoundboardForMe, 1);
        // The user moved Voicemod's own slider since
        mixer.voicemod_gain(Channel::SoundboardForMe, gain(70), false);
        assert_eq!(mixer.step_gain(Channel::SoundboardForMe, 1), Some(75));
        assert_eq!(mixer.reset().channels, [(Channel::SoundboardForMe, gain(70))]);
    }

    #[test]
    fn a_sounds_volume_lasts_one_play() {
        let mut mixer = mixer();
        assert_eq!(mixer.step_sound("song", 1), Some(35));
        assert_eq!(mixer.step_sound("song", 1), Some(40));
        assert_eq!(mixer.step_sound("honk", -1), Some(95));
        mixer.keep_playing(|sound| sound == "honk");
        assert_eq!(
            mixer.step_sound("song", 1),
            Some(35),
            "ended: from its saved volume again"
        );
        assert_eq!(mixer.step_sound("honk", -1), Some(90), "still playing");
        assert_eq!(
            mixer.reset().sounds,
            [("honk".to_string(), 1.0), ("song".to_string(), 0.3)]
        );
        assert_eq!(mixer.step_sound("song", -1), Some(25));
    }

    #[test]
    fn new_saved_volumes_keep_the_play_under_way() {
        let mut mixer = mixer();
        mixer.step_sound("song", 2);
        mixer.sound_volumes(HashMap::from([("song".into(), 0.5)]));
        assert_eq!(mixer.step_sound("song", 1), Some(45));
        assert_eq!(mixer.reset().sounds, [("song".to_string(), 0.5)]);
    }

    #[test]
    fn a_new_connection_forgets_the_old_values() {
        let mut mixer = mixer();
        mixer.step_gain(Channel::SoundboardForOthers, -1);
        mixer.step_sound("song", 1);
        mixer.connected();
        assert_eq!(mixer.step_gain(Channel::SoundboardForOthers, 1), None);
        assert_eq!(mixer.step_sound("song", 1), None);
        assert_eq!(mixer.reset(), Reset::default());
    }
}
