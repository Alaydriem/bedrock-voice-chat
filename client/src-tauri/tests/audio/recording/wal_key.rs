use bvc_client_lib::audio::recording::WalKey;

#[test]
fn sanitize_drops_the_colon_that_ntfs_reads_as_a_stream_separator() {
    assert_eq!(WalKey::sanitize("minecraft:Alaydriem"), "minecraftAlaydriem");
}

#[test]
fn sanitize_truncates_to_the_length_the_wal_uses() {
    assert_eq!(WalKey::sanitize("abcdefghijklmnopqrstuvwxyz").len(), 20);
}

// A short key must not claim a longer key's segments. The separator is what makes the
// boundary, and without it "Al" reads everything Alaydriem recorded.
#[test]
fn a_key_does_not_match_a_longer_keys_segments() {
    assert!(!WalKey::matches("Alaydriem-9f2c-0.log", "Al"));
    assert!(WalKey::matches("Alaydriem-9f2c-0.log", "Alaydriem"));
}

#[test]
fn matching_ignores_files_that_are_not_segments() {
    assert!(!WalKey::matches("Alaydriem-9f2c-0.tmp", "Alaydriem"));
}

#[test]
fn matching_applies_the_same_sanitising_as_writing() {
    assert!(WalKey::matches(
        "minecraftAlaydriem-9f2c-0.log",
        "minecraft:Alaydriem"
    ));
}
