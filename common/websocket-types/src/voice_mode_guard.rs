use super::{Command, DeviceType, VoiceMode};

/// Which commands mean anything in which voice mode.
///
/// The rule is one sentence and it decides what a controller draws: in push-to-talk the
/// hold owns the microphone, so an input mute is not the caller's to issue and a toggle
/// beside the hold is a second word for a state the hold already has. It lives here, beside
/// the schema, because every client needs the same answer — the one that absorbed a refused
/// mute and reported success left controllers drawing a button that did nothing.
pub struct VoiceModeGuard;

impl VoiceModeGuard {
    /// Whether a mute for `device` is the caller's to issue.
    ///
    /// Output is always theirs: deafening is unrelated to how transmission is triggered.
    pub fn allows_mute(mode: VoiceMode, device: &DeviceType) -> bool {
        match device {
            DeviceType::Output => true,
            DeviceType::Input => mode == VoiceMode::OpenMic,
        }
    }

    /// Whether a hold means anything. Honoured in open mic it would be a remote command
    /// that silently opens the microphone.
    pub fn allows_ptt(mode: VoiceMode) -> bool {
        mode == VoiceMode::PushToTalk
    }

    /// Why a command was refused, or `None` when it was not.
    pub fn refusal(mode: VoiceMode, command: &Command) -> Option<String> {
        match command {
            Command::Mute { device } if !Self::allows_mute(mode, device) => Some(
                "input mute is governed by push-to-talk; send the ptt action with down true or false instead"
                    .to_string(),
            ),
            Command::Ptt { .. } if !Self::allows_ptt(mode) => {
                Some("push-to-talk is not the current voice mode".to_string())
            }
            _ => None,
        }
    }
}
