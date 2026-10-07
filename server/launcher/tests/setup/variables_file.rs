use std::fs;

use bvc_launcher::setup::VariablesFile;

#[test]
fn absent_null_and_blank_values_are_missing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("variables.json");
    fs::write(&path, r#"{"bvc_server":"  ","bvc_access_token":null}"#).unwrap();

    let variables = VariablesFile::load(&path).unwrap();

    assert!(variables.is_missing(VariablesFile::SERVER_KEY));
    assert!(variables.is_missing(VariablesFile::TOKEN_KEY));
    assert!(variables.is_missing("bvc_world_name"));
}

#[test]
fn a_set_value_is_not_missing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("variables.json");
    fs::write(&path, r#"{"bvc_access_token":"bvc_abcdefgh_secret"}"#).unwrap();

    assert!(!VariablesFile::load(&path).unwrap().is_missing(VariablesFile::TOKEN_KEY));
}

#[test]
fn saving_keeps_other_keys_their_types_and_their_order() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config").join("default").join("variables.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        r#"{"bvc_world_name":"SMP","bvc_minimum_players":2,"bvc_server":""}"#,
    )
    .unwrap();

    let mut variables = VariablesFile::load(&path).unwrap();
    variables.set(VariablesFile::SERVER_KEY, "https://voice.example.com");
    variables.set(VariablesFile::TOKEN_KEY, "bvc_abcdefgh_secret");
    variables.save().unwrap();

    let written: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let keys: Vec<&str> = written.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        vec!["bvc_world_name", "bvc_minimum_players", "bvc_server", "bvc_access_token"]
    );
    assert_eq!(written["bvc_minimum_players"], serde_json::json!(2));
    assert_eq!(written["bvc_server"], serde_json::json!("https://voice.example.com"));
}

#[test]
fn a_missing_file_loads_empty_and_saves_with_its_directories() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config").join("default").join("variables.json");

    let mut variables = VariablesFile::load(&path).unwrap();
    assert!(variables.is_missing(VariablesFile::SERVER_KEY));
    variables.set(VariablesFile::SERVER_KEY, "https://voice.example.com");
    variables.save().unwrap();

    assert!(fs::read_to_string(&path).unwrap().contains("https://voice.example.com"));
}

#[test]
fn invalid_json_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("variables.json");
    fs::write(&path, "{").unwrap();

    assert!(VariablesFile::load(&path).is_err());
}
