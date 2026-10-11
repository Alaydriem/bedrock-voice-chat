use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use bvc_launcher::setup::WorldPatcher;
use zuri_nbt::tag::Byte;
use zuri_nbt::{NBTRoot, NBTTag};

struct LevelFixture;

impl LevelFixture {
    const SOURCE: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/level.dat");

    fn source_version() -> i32 {
        WorldPatcher::decode(&fs::read(Self::SOURCE).unwrap()).unwrap().0
    }

    // A copy of the fixture in `dir`, with `edit` applied to its top-level compound.
    fn copy_with(dir: &Path, edit: impl FnOnce(&mut HashMap<String, NBTTag>)) -> PathBuf {
        let (version, mut root) = WorldPatcher::decode(&fs::read(Self::SOURCE).unwrap()).unwrap();
        edit(Self::top(&mut root));
        let path = dir.join("level.dat");
        fs::write(&path, WorldPatcher::encode(version, &root).unwrap()).unwrap();
        path
    }

    fn top(root: &mut NBTRoot) -> &mut HashMap<String, NBTTag> {
        match &mut root.data {
            NBTTag::Compound(compound) => &mut compound.0,
            _ => panic!("level.dat root is not a compound"),
        }
    }

    fn read(path: &Path) -> (i32, NBTRoot) {
        WorldPatcher::decode(&fs::read(path).unwrap()).unwrap()
    }

    fn without_gametest(top: &mut HashMap<String, NBTTag>) {
        if let Some(NBTTag::Compound(experiments)) = top.get_mut("experiments") {
            experiments.0.remove("gametest");
        }
    }

    fn take_experiments(top: &mut HashMap<String, NBTTag>) -> HashMap<String, NBTTag> {
        match top.remove("experiments") {
            Some(NBTTag::Compound(experiments)) => experiments.0,
            _ => HashMap::new(),
        }
    }
}

#[test]
fn enabling_beta_apis_changes_only_the_gametest_flag() {
    let dir = tempfile::tempdir().unwrap();
    let path = LevelFixture::copy_with(dir.path(), LevelFixture::without_gametest);
    let (_, mut before) = LevelFixture::read(&path);

    assert!(WorldPatcher::enable_beta_apis(&path).unwrap());

    let (version, mut after) = LevelFixture::read(&path);
    assert_eq!(version, LevelFixture::source_version());

    let after_top = LevelFixture::top(&mut after);
    let mut after_experiments = LevelFixture::take_experiments(after_top);
    assert_eq!(after_experiments.remove("gametest"), Some(NBTTag::Byte(Byte(1))));

    let before_top = LevelFixture::top(&mut before);
    let before_experiments = LevelFixture::take_experiments(before_top);
    assert_eq!(after_experiments, before_experiments);
    assert_eq!(after_top, before_top);
}

#[test]
fn a_second_run_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let path = LevelFixture::copy_with(dir.path(), LevelFixture::without_gametest);
    WorldPatcher::enable_beta_apis(&path).unwrap();
    let patched = fs::read(&path).unwrap();

    assert!(!WorldPatcher::enable_beta_apis(&path).unwrap());
    assert_eq!(fs::read(&path).unwrap(), patched);
}

#[test]
fn a_world_without_experiments_gets_only_the_gametest_flag() {
    let dir = tempfile::tempdir().unwrap();
    let path = LevelFixture::copy_with(dir.path(), |top| {
        top.remove("experiments");
    });

    assert!(WorldPatcher::enable_beta_apis(&path).unwrap());

    let (_, mut after) = LevelFixture::read(&path);
    let experiments = LevelFixture::take_experiments(LevelFixture::top(&mut after));
    assert_eq!(
        experiments,
        HashMap::from([("gametest".to_string(), NBTTag::Byte(Byte(1)))])
    );
}

#[test]
fn a_truncated_file_is_an_error_and_is_left_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("level.dat");
    let full = fs::read(LevelFixture::SOURCE).unwrap();
    let truncated = &full[..full.len() / 2];
    fs::write(&path, truncated).unwrap();

    assert!(WorldPatcher::enable_beta_apis(&path).is_err());
    assert_eq!(fs::read(&path).unwrap(), truncated);
}

#[test]
fn a_file_shorter_than_its_header_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("level.dat");
    fs::write(&path, [1, 2, 3, 4, 5]).unwrap();

    assert!(WorldPatcher::enable_beta_apis(&path).is_err());
    assert_eq!(fs::read(&path).unwrap(), vec![1, 2, 3, 4, 5]);
}
