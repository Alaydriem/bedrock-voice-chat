use std::fs;

use bvc_launcher::setup::AllowlistFile;

#[test]
fn reads_each_name_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("allowlist.json");
    fs::write(
        &path,
        r#"[{"ignoresPlayerLimit":false,"name":"Alaydriem"},{"name":"Some Player","xuid":"1"}]"#,
    )
    .unwrap();

    let allowlist = AllowlistFile::load(&path).unwrap();

    assert_eq!(allowlist.names(), ["Alaydriem", "Some Player"]);
    assert_eq!(allowlist.skipped(), 0);
}

#[test]
fn entries_without_a_usable_name_are_skipped_and_counted() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("allowlist.json");
    fs::write(&path, r#"[{"name":"Alaydriem"},{},{"name":"  "},{"name":7},"x"]"#).unwrap();

    let allowlist = AllowlistFile::load(&path).unwrap();

    assert_eq!(allowlist.names(), ["Alaydriem"]);
    assert_eq!(allowlist.skipped(), 4);
}

#[test]
fn a_missing_file_is_an_error() {
    let dir = tempfile::tempdir().unwrap();

    assert!(AllowlistFile::load(&dir.path().join("allowlist.json")).is_err());
}

#[test]
fn a_file_that_is_not_an_array_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("allowlist.json");
    fs::write(&path, r#"{"name":"Alaydriem"}"#).unwrap();

    assert!(AllowlistFile::load(&path).is_err());
}
