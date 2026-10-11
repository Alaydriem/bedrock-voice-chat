use std::fs;

use bvc_launcher::setup::{BvcPackIds, PackIdentity, WorldPacksFile};
use serde_json::{Value, json};

struct Packs;

impl Packs {
    const NO_NET_BEHAVIOR: &'static str = "508a43ed-b95e-40cf-98eb-d78f2d72caa4";
    const OTHER: &'static str = "11111111-2222-3333-4444-555555555555";

    fn behavior(version: [u64; 3]) -> PackIdentity {
        PackIdentity {
            uuid: "6fb24263-357a-407c-aebe-681f50b2de50".to_string(),
            version: version.to_vec(),
        }
    }

    fn read(path: &std::path::Path) -> Value {
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    }
}

#[test]
fn a_missing_file_is_created_with_the_pack() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("world_behavior_packs.json");

    let changed = WorldPacksFile::new(path.clone())
        .register(&Packs::behavior([1, 0, 521]), &BvcPackIds::BEHAVIOR)
        .unwrap();

    assert!(changed);
    assert_eq!(
        Packs::read(&path),
        json!([{"pack_id": "6fb24263-357a-407c-aebe-681f50b2de50", "version": [1, 0, 521]}])
    );
}

#[test]
fn an_older_bvc_entry_is_updated_in_place_and_other_packs_are_kept() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("world_behavior_packs.json");
    fs::write(
        &path,
        json!([
            {"pack_id": Packs::OTHER, "version": [2, 0, 0]},
            {"pack_id": "6fb24263-357a-407c-aebe-681f50b2de50", "version": [1, 0, 400]},
            {"pack_id": Packs::NO_NET_BEHAVIOR, "version": [1, 0, 400]}
        ])
        .to_string(),
    )
    .unwrap();

    WorldPacksFile::new(path.clone())
        .register(&Packs::behavior([1, 0, 521]), &BvcPackIds::BEHAVIOR)
        .unwrap();

    assert_eq!(
        Packs::read(&path),
        json!([
            {"pack_id": Packs::OTHER, "version": [2, 0, 0]},
            {"pack_id": "6fb24263-357a-407c-aebe-681f50b2de50", "version": [1, 0, 521]}
        ])
    );
}

#[test]
fn a_no_net_entry_is_replaced_by_the_installed_pack() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("world_behavior_packs.json");
    fs::write(
        &path,
        json!([{"pack_id": Packs::NO_NET_BEHAVIOR, "version": [1, 0, 400]}]).to_string(),
    )
    .unwrap();

    WorldPacksFile::new(path.clone())
        .register(&Packs::behavior([1, 0, 521]), &BvcPackIds::BEHAVIOR)
        .unwrap();

    assert_eq!(
        Packs::read(&path),
        json!([{"pack_id": "6fb24263-357a-407c-aebe-681f50b2de50", "version": [1, 0, 521]}])
    );
}

#[test]
fn a_current_entry_is_not_rewritten() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("world_behavior_packs.json");
    let original =
        r#"[{"pack_id":"6fb24263-357a-407c-aebe-681f50b2de50","version":[1,0,521]}]"#;
    fs::write(&path, original).unwrap();

    let changed = WorldPacksFile::new(path.clone())
        .register(&Packs::behavior([1, 0, 521]), &BvcPackIds::BEHAVIOR)
        .unwrap();

    assert!(!changed);
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
}

#[test]
fn a_file_that_is_not_an_array_is_an_error_and_is_left_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("world_behavior_packs.json");
    let original = r#"{"pack_id":"6fb24263-357a-407c-aebe-681f50b2de50"}"#;
    fs::write(&path, original).unwrap();

    let result = WorldPacksFile::new(path.clone())
        .register(&Packs::behavior([1, 0, 521]), &BvcPackIds::BEHAVIOR);

    assert!(result.is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
}

#[test]
fn invalid_json_is_an_error_and_the_file_is_left_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("world_behavior_packs.json");
    let original = r#"[{"pack_id": "x",]"#;
    fs::write(&path, original).unwrap();

    let result = WorldPacksFile::new(path.clone())
        .register(&Packs::behavior([1, 0, 521]), &BvcPackIds::BEHAVIOR);

    assert!(result.is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
}
