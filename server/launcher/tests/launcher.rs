use std::fs;

use bvc_launcher::Launcher;
use bvc_launcher::release::ReleaseSources;

#[test]
fn a_missing_config_exits_with_code_1_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();

    let code = Launcher::new(dir.path().to_path_buf(), Vec::new(), ReleaseSources::default()).run();

    assert_eq!(code, 1);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[cfg(target_os = "linux")]
mod linux {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    use bvc_launcher::release::ReleaseSources;
    use bvc_launcher::setup::WorldPatcher;
    use bvc_launcher::{Launcher, LauncherPaths};
    use serde_json::{Value, json};
    use zuri_nbt::NBTTag;
    use zuri_nbt::tag::Byte;

    use crate::support::{Addon, HttpFixture};

    /// A BDS directory with config, the released add-on on disk, and a BVC that cannot start,
    /// so the launcher runs offline and mints no token.
    struct Server {
        paths: LauncherPaths,
        // Kept alive: the launcher's update step dials it and gets 404s.
        http: HttpFixture,
    }

    impl Server {
        const LEVEL: &'static str = "Bedrock level";

        fn prepare(root: &Path, bds_body: &str) -> Self {
            let paths = LauncherPaths::new(root.to_path_buf());
            fs::create_dir_all(paths.bvc_binary().parent().unwrap()).unwrap();
            fs::create_dir_all(paths.pack_file().parent().unwrap()).unwrap();
            fs::write(
                paths.config_file(),
                "server {\n  tls {\n    names = [\"voice.example.com\"]\n  }\n}\n",
            )
            .unwrap();
            fs::write(paths.bvc_binary(), "#!/bin/sh\nexit 0\n").unwrap();
            fs::set_permissions(paths.bvc_binary(), fs::Permissions::from_mode(0o644)).unwrap();
            fs::write(paths.bds_binary(), format!("#!/bin/sh\n{bds_body}\n")).unwrap();
            fs::set_permissions(paths.bds_binary(), fs::Permissions::from_mode(0o755)).unwrap();
            Addon::released(&paths.pack_file());
            Self {
                paths,
                http: HttpFixture::start(),
            }
        }

        fn sources(&self) -> ReleaseSources {
            ReleaseSources::new(
                vec![self.http.url("/latest.json")],
                self.http.url("/releases/download"),
                Duration::from_secs(2),
            )
        }

        fn world_dir(&self) -> PathBuf {
            self.paths.level_dat(Self::LEVEL).parent().unwrap().to_path_buf()
        }

        // A copy of the real level.dat fixture without the Beta APIs flag.
        fn level_dat_without_beta_apis(dir: &Path) -> PathBuf {
            let source = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/level.dat");
            let (version, mut root) = WorldPatcher::decode(&fs::read(source).unwrap()).unwrap();
            if let NBTTag::Compound(top) = &mut root.data
                && let Some(NBTTag::Compound(experiments)) = top.0.get_mut("experiments")
            {
                experiments.0.remove("gametest");
            }
            let path = dir.join("fresh-level.dat");
            fs::write(&path, WorldPatcher::encode(version, &root).unwrap()).unwrap();
            path
        }

        fn has_beta_apis(level_dat: &Path) -> bool {
            let (_, root) = WorldPatcher::decode(&fs::read(level_dat).unwrap()).unwrap();
            let NBTTag::Compound(top) = root.data else {
                return false;
            };
            matches!(
                top.0.get("experiments"),
                Some(NBTTag::Compound(e)) if e.0.get("gametest") == Some(&NBTTag::Byte(Byte(1)))
            )
        }

        fn assert_packs_registered(&self) {
            let read = |name: &str| -> Value {
                serde_json::from_str(&fs::read_to_string(self.world_dir().join(name)).unwrap())
                    .unwrap()
            };
            assert_eq!(
                read("world_behavior_packs.json"),
                json!([{"pack_id": Addon::BEHAVIOR_UUID, "version": Addon::VERSION}])
            );
            assert_eq!(
                read("world_resource_packs.json"),
                json!([{"pack_id": Addon::RESOURCE_UUID, "version": Addon::VERSION}])
            );
        }
    }

    #[test]
    fn a_bvc_that_cannot_start_does_not_hold_bds_for_token_retries() {
        let root = tempfile::tempdir().unwrap();
        let server = Server::prepare(root.path(), "exit 7");
        let level_dat = server.paths.level_dat(Server::LEVEL);
        fs::create_dir_all(level_dat.parent().unwrap()).unwrap();
        fs::copy(Server::level_dat_without_beta_apis(root.path()), &level_dat).unwrap();
        let started = Instant::now();

        let code = Launcher::new(root.path().to_path_buf(), Vec::new(), server.sources()).run();

        assert_eq!(code, 7);
        assert!(started.elapsed() < Duration::from_secs(15));
        let variables = fs::read_to_string(server.paths.variables_file()).unwrap();
        assert!(variables.contains("https://voice.example.com"));
        assert!(!variables.contains("bvc_access_token"));
        assert!(Server::has_beta_apis(&level_dat));
        server.assert_packs_registered();
    }

    #[test]
    fn a_new_world_is_patched_and_registered_after_its_first_start() {
        let root = tempfile::tempdir().unwrap();
        let fresh = Server::level_dat_without_beta_apis(root.path());
        // First start: create the world, then wait for `stop`. Second start: exit 5.
        let bds = format!(
            "W=\"worlds/{}\"\nif [ -f \"$W/level.dat\" ]; then exit 5; fi\nmkdir -p \"$W\"\ncp \"{}\" \"$W/level.dat\"\nwhile read line; do [ \"$line\" = \"stop\" ] && exit 0; done",
            Server::LEVEL,
            fresh.display()
        );
        let server = Server::prepare(root.path(), &bds);

        let code = Launcher::new(root.path().to_path_buf(), Vec::new(), server.sources()).run();

        assert_eq!(code, 5);
        assert!(Server::has_beta_apis(&server.paths.level_dat(Server::LEVEL)));
        server.assert_packs_registered();
    }
}
