use websocket_types::{Command, DeviceType, VoiceMode, VoiceModeGuard};

/// The bug this rule exists to prevent.
///
/// Input mute in push-to-talk used to be absorbed: the client answered with the current
/// state and changed nothing, so a controller drew a mute button that reported success and
/// did nothing, forever. The hold is the control in this mode.
#[test]
fn refuses_an_input_mute_in_push_to_talk() {
    let mute = Command::Mute {
        device: DeviceType::Input,
    };
    assert!(!VoiceModeGuard::allows_mute(
        VoiceMode::PushToTalk,
        &DeviceType::Input
    ));

    let reason = VoiceModeGuard::refusal(VoiceMode::PushToTalk, &mute).expect("refused");
    assert!(reason.contains("ptt"), "the refusal names the way in: {reason}");
}

#[test]
fn allows_an_input_mute_in_open_mic() {
    assert!(VoiceModeGuard::allows_mute(
        VoiceMode::OpenMic,
        &DeviceType::Input
    ));
    assert!(
        VoiceModeGuard::refusal(
            VoiceMode::OpenMic,
            &Command::Mute {
                device: DeviceType::Input
            }
        )
        .is_none()
    );
}

/// Deafening is unrelated to how transmission is triggered. Refusing it in push-to-talk
/// would take away the one control that still means what it says.
#[test]
fn never_refuses_an_output_mute() {
    for mode in [VoiceMode::OpenMic, VoiceMode::PushToTalk] {
        assert!(VoiceModeGuard::allows_mute(mode, &DeviceType::Output));
        assert!(
            VoiceModeGuard::refusal(
                mode,
                &Command::Mute {
                    device: DeviceType::Output
                }
            )
            .is_none()
        );
    }
}

/// Honoured in open mic, a hold from a controller is a remote command that opens the
/// microphone without anything on screen asking for it.
#[test]
fn refuses_a_hold_outside_push_to_talk() {
    assert!(!VoiceModeGuard::allows_ptt(VoiceMode::OpenMic));
    assert!(VoiceModeGuard::refusal(VoiceMode::OpenMic, &Command::Ptt { down: true }).is_some());
}

#[test]
fn allows_a_hold_in_push_to_talk() {
    assert!(VoiceModeGuard::allows_ptt(VoiceMode::PushToTalk));
    for down in [true, false] {
        assert!(VoiceModeGuard::refusal(VoiceMode::PushToTalk, &Command::Ptt { down }).is_none());
    }
}

/// A release must never be refused by the same rule that refuses a press, or a mode changed
/// mid-hold would leave the microphone open with no command able to close it.
#[test]
fn treats_a_release_exactly_as_a_press() {
    for mode in [VoiceMode::OpenMic, VoiceMode::PushToTalk] {
        let press = VoiceModeGuard::refusal(mode, &Command::Ptt { down: true }).is_some();
        let release = VoiceModeGuard::refusal(mode, &Command::Ptt { down: false }).is_some();
        assert_eq!(press, release, "press and release disagree in {mode:?}");
    }
}

#[test]
fn refuses_nothing_else() {
    for mode in [VoiceMode::OpenMic, VoiceMode::PushToTalk] {
        assert!(VoiceModeGuard::refusal(mode, &Command::Ping).is_none());
        assert!(VoiceModeGuard::refusal(mode, &Command::Record).is_none());
        assert!(VoiceModeGuard::refusal(mode, &Command::State).is_none());
    }
}
