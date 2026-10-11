use std::fs;

use bvc_launcher::setup::PackInstaller;

use crate::support::Addon;

#[test]
fn installs_each_pack_unextracted_and_reports_its_identity() {
    let dir = tempfile::tempdir().unwrap();
    let addon = dir.path().join("bvc.mcaddon");
    let behavior_pack = Addon::mcpack(Addon::BEHAVIOR_UUID, [1, 0, 521]);
    let resource_pack = Addon::mcpack(Addon::RESOURCE_UUID, [1, 0, 522]);
    Addon::build(
        &addon,
        &[
            ("bedrock-voice-chat-bp.mcpack", &behavior_pack),
            ("bedrock-voice-chat-rp.mcpack", &resource_pack),
        ],
    );
    let behavior = dir.path().join("development_behavior_packs");
    let resource = dir.path().join("development_resource_packs");

    let installed = PackInstaller::new(behavior.clone(), resource.clone())
        .install(&addon)
        .unwrap();

    assert_eq!(fs::read(behavior.join("bedrock-voice-chat-bp.mcpack")).unwrap(), behavior_pack);
    assert_eq!(fs::read(resource.join("bedrock-voice-chat-rp.mcpack")).unwrap(), resource_pack);
    assert_eq!(installed.behavior.uuid, Addon::BEHAVIOR_UUID);
    assert_eq!(installed.behavior.version, vec![1, 0, 521]);
    assert_eq!(installed.resource.uuid, Addon::RESOURCE_UUID);
    assert_eq!(installed.resource.version, vec![1, 0, 522]);
}

#[test]
fn replaces_older_bvc_packs_and_keeps_other_packs() {
    let dir = tempfile::tempdir().unwrap();
    let addon = dir.path().join("bvc.mcaddon");
    Addon::released(&addon);
    let behavior = dir.path().join("development_behavior_packs");
    let resource = dir.path().join("development_resource_packs");
    fs::create_dir_all(behavior.join("bedrock-voice-chat-bp-0.0.1")).unwrap();
    fs::create_dir_all(&resource).unwrap();
    fs::write(behavior.join("bedrock-voice-chat-bp.zip"), b"old").unwrap();
    fs::write(behavior.join("bedrock-voice-chat-bp.mcpack"), b"old").unwrap();
    fs::write(behavior.join("other-pack.mcpack"), b"other").unwrap();
    fs::write(resource.join("bedrock-voice-chat-rp.zip"), b"old").unwrap();

    PackInstaller::new(behavior.clone(), resource.clone())
        .install(&addon)
        .unwrap();

    let mut behavior_names: Vec<String> = fs::read_dir(&behavior)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    behavior_names.sort();
    assert_eq!(behavior_names, vec!["bedrock-voice-chat-bp.mcpack", "other-pack.mcpack"]);
    assert_ne!(fs::read(behavior.join("bedrock-voice-chat-bp.mcpack")).unwrap(), b"old");
    assert!(!resource.join("bedrock-voice-chat-rp.zip").exists());
}

#[test]
fn an_addon_missing_a_pack_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let addon = dir.path().join("bvc.mcaddon");
    let behavior_pack = Addon::mcpack(Addon::BEHAVIOR_UUID, Addon::VERSION);
    Addon::build(&addon, &[("bedrock-voice-chat-bp.mcpack", &behavior_pack)]);
    let behavior = dir.path().join("development_behavior_packs");
    let resource = dir.path().join("development_resource_packs");
    fs::create_dir_all(&behavior).unwrap();
    fs::write(behavior.join("bedrock-voice-chat-bp.zip"), b"old").unwrap();

    assert!(PackInstaller::new(behavior.clone(), resource.clone()).install(&addon).is_err());

    assert_eq!(fs::read(behavior.join("bedrock-voice-chat-bp.zip")).unwrap(), b"old");
    assert!(!behavior.join("bedrock-voice-chat-bp.mcpack").exists());
    assert!(!resource.exists());
}

#[test]
fn a_pack_without_a_manifest_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let addon = dir.path().join("bvc.mcaddon");
    let resource_pack = Addon::mcpack(Addon::RESOURCE_UUID, Addon::VERSION);
    Addon::build(
        &addon,
        &[
            ("bedrock-voice-chat-bp.mcpack", b"not a pack"),
            ("bedrock-voice-chat-rp.mcpack", &resource_pack),
        ],
    );
    let behavior = dir.path().join("development_behavior_packs");
    let resource = dir.path().join("development_resource_packs");

    assert!(PackInstaller::new(behavior.clone(), resource.clone()).install(&addon).is_err());

    assert!(!behavior.exists());
    assert!(!resource.exists());
}
