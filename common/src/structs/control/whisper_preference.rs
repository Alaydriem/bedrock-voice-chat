use super::PlayerPreference;
use crate::consts::audio::WHISPER_CONTROL_TARGET;

/// The crouch-to-whisper choice, as the gain-shaped preference the plane carries.
///
/// The one place that reads or writes the mapping: `muted` is the choice, read as "crouching
/// mutes me past the whisper range", and `volume` is unused.
pub struct WhisperPreference;

impl WhisperPreference {
    pub fn for_owner(owner: impl Into<String>, enabled: bool) -> PlayerPreference {
        PlayerPreference {
            owner: owner.into(),
            target: WHISPER_CONTROL_TARGET.to_string(),
            volume: 1.0,
            muted: enabled,
        }
    }

    pub fn is_enabled(preference: &PlayerPreference) -> bool {
        preference.target == WHISPER_CONTROL_TARGET && preference.muted
    }
}
