use bvc_client_lib::{KeyringFault, KeyringFaultKind};

// The three dbus-secret-service messages that mean the OS keyring needs attention before
// anything can be written. Pinned as literals: the classifier reads a rendered message, so a
// change in the upstream wording has to fail here rather than silently reclassify every Linux
// write failure as a generic platform error.
#[test]
fn the_keyring_unusable_messages_are_classified_as_unusable() {
    for message in [
        "Platform error: Couldn't access platform storage: Secret Service: no result found",
        "Platform error: Couldn't access platform storage: Secret Service: object locked",
        "Platform error: Couldn't access platform storage: Secret Service: unlock prompt was dismissed",
    ] {
        assert_eq!(
            KeyringFault::classify(message),
            KeyringFaultKind::Unusable,
            "{message:?} should be an unusable keyring"
        );
    }
}

// A missing session bus or provider is a launch failure, not a write failure: the store connects
// eagerly in the plugin's init, so this message cannot reach a credential write. Classifying it
// as Unusable would send someone to create a keyring for a problem that is not theirs to fix.
#[test]
fn an_unrelated_platform_error_is_classified_as_other() {
    for message in [
        "Platform error: Couldn't access platform storage: no space left on device",
        "Failed to set keyring password for gamerpic: Entry not found in keyring",
        "Platform error: access is denied",
        "",
    ] {
        assert_eq!(
            KeyringFault::classify(message),
            KeyringFaultKind::Other,
            "{message:?} should not be an unusable keyring"
        );
    }
}

// The two codes the error route renders. A kind that mapped to a code with no catalogue entry
// would show an empty screen, which is worse than the generic one.
#[test]
fn each_kind_maps_to_its_fault_code() {
    assert_eq!(KeyringFault::code(KeyringFaultKind::Unusable), "AUTH04");
    assert_eq!(KeyringFault::code(KeyringFaultKind::Other), "AUTH03");
}

// Matching is case-insensitive because the message is assembled from three layers - keyring-core,
// the store crate and this crate - and none of them agree on capitalisation.
#[test]
fn classification_ignores_case() {
    assert_eq!(
        KeyringFault::classify("SECRET SERVICE: NO RESULT FOUND"),
        KeyringFaultKind::Unusable
    );
}

#[test]
fn label_returns_the_code_for_the_message() {
    assert_eq!(
        KeyringFault::label("Secret Service: no result found"),
        "AUTH04"
    );
    assert_eq!(KeyringFault::label("no space left on device"), "AUTH03");
}
