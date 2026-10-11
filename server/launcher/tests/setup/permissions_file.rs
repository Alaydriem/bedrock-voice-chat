use std::fs;

use bvc_launcher::setup::PermissionsFile;
use serde_json::Value;

#[test]
fn creates_the_file_with_the_required_modules() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config").join("default").join("permissions.json");

    assert!(PermissionsFile::new(path.clone()).ensure_required_modules().unwrap());

    let written: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let modules: Vec<&str> = written["allowed_modules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(modules, PermissionsFile::REQUIRED_MODULES.to_vec());
}

#[test]
fn keeps_existing_modules_and_keys_without_duplicates() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("permissions.json");
    fs::write(
        &path,
        r#"{"allowed_modules":["@minecraft/server-gametest","@minecraft/server"],"other":true}"#,
    )
    .unwrap();

    assert!(PermissionsFile::new(path.clone()).ensure_required_modules().unwrap());

    let written: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let modules: Vec<&str> = written["allowed_modules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(
        modules,
        vec![
            "@minecraft/server-gametest",
            "@minecraft/server",
            "@minecraft/server-ui",
            "@minecraft/server-net",
            "@minecraft/server-admin",
        ]
    );
    assert_eq!(written["other"], Value::Bool(true));
}

#[test]
fn a_complete_file_is_not_rewritten() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("permissions.json");
    let original = r#"{"allowed_modules":["@minecraft/server","@minecraft/server-ui","@minecraft/server-net","@minecraft/server-admin"]}"#;
    fs::write(&path, original).unwrap();

    assert!(!PermissionsFile::new(path.clone()).ensure_required_modules().unwrap());
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
}

#[test]
fn a_non_array_allowed_modules_is_an_error_and_the_file_is_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("permissions.json");
    let original = r#"{"allowed_modules":"@minecraft/server"}"#;
    fs::write(&path, original).unwrap();

    assert!(PermissionsFile::new(path.clone()).ensure_required_modules().is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
}

#[test]
fn invalid_json_is_an_error_and_the_file_is_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("permissions.json");
    let original = r#"{"allowed_modules": ["@minecraft/server",]"#;
    fs::write(&path, original).unwrap();

    assert!(PermissionsFile::new(path.clone()).ensure_required_modules().is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
}
