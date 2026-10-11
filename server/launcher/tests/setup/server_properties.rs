use std::fs;

use bvc_launcher::setup::ServerProperties;

#[test]
fn reads_level_name_with_spaces() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("server.properties");
    fs::write(
        &path,
        "# level-name=commented\nserver-name=Test\nlevel-name=BDS Test World\n",
    )
    .unwrap();

    assert_eq!(ServerProperties::level_name(&path), "BDS Test World");
}

#[test]
fn defaults_to_the_bds_default_when_unset() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("server.properties");
    fs::write(&path, "server-name=Test\n").unwrap();

    assert_eq!(ServerProperties::level_name(&path), "Bedrock level");
}

#[test]
fn defaults_to_the_bds_default_when_the_file_is_missing() {
    let dir = tempfile::tempdir().unwrap();

    assert_eq!(
        ServerProperties::level_name(&dir.path().join("server.properties")),
        "Bedrock level"
    );
}
