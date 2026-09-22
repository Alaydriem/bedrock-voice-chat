use bvc_client_lib::audio::AudioActionsManager;

// Deafening drives the microphone as well as the speakers. Hearing nobody while they can
// still hear you is a state people reach by accident and cannot detect from their own
// screen, so every surface that deafens has to close the input too.
#[test]
fn deafening_shuts_the_microphone() {
    assert!(AudioActionsManager::deafen_input_target(true, false));
}

// Push-to-talk does not exempt the deafen. The resting state is already shut, and the
// answer has to be the same one open mic gives so the two modes cannot disagree about
// what deafen means.
#[test]
fn deafening_shuts_the_microphone_in_push_to_talk() {
    assert!(AudioActionsManager::deafen_input_target(true, true));
}

// Undeafening clears both, because the fix for "I cannot hear anyone" must not be a second
// button the user has no reason to suspect.
#[test]
fn undeafening_opens_the_microphone_in_open_mic() {
    assert!(!AudioActionsManager::deafen_input_target(false, false));
}

// Except in push-to-talk, where an open microphone is the wrong resting state. Undeafening
// used to open it unconditionally, and the mic button, which reads the same flag, then drew
// the opposite of the mode it was in.
#[test]
fn undeafening_leaves_the_microphone_shut_in_push_to_talk() {
    assert!(AudioActionsManager::deafen_input_target(false, true));
}
