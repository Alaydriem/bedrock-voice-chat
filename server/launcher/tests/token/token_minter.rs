use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use bvc_launcher::token::TokenMinter;

struct FakeBvc;

impl FakeBvc {
    fn script(dir: &Path, body: &str) -> PathBuf {
        let path = dir.join("bvc-server");
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    fn minter(binary: PathBuf, dir: &Path, attempts: u32, delay: Duration) -> TokenMinter {
        TokenMinter::new(
            binary,
            dir.to_path_buf(),
            dir.join("config").join("bvc.hcl"),
            attempts,
            delay,
        )
    }
}

#[test]
fn returns_the_trimmed_token_once_bvc_is_ready() {
    let dir = tempfile::tempdir().unwrap();
    let binary = FakeBvc::script(
        dir.path(),
        r#"n=$(cat count 2>/dev/null || echo 0); n=$((n+1)); echo $n > count
if [ $n -lt 3 ]; then echo "no such table: game_access_token" >&2; exit 1; fi
echo "  bvc_abcdefgh_secret  ""#,
    );

    let token = FakeBvc::minter(binary, dir.path(), 5, Duration::from_millis(10))
        .mint()
        .unwrap();

    assert_eq!(token, "bvc_abcdefgh_secret");
}

#[test]
fn passes_the_config_and_local_mint_arguments_from_the_bvc_directory() {
    let dir = tempfile::tempdir().unwrap();
    let binary = FakeBvc::script(dir.path(), r#"[ -f marker ] && echo "$@""#);
    fs::write(dir.path().join("marker"), "").unwrap();

    let output = FakeBvc::minter(binary, dir.path(), 1, Duration::ZERO)
        .mint()
        .unwrap();

    let config = dir.path().join("config").join("bvc.hcl");
    assert_eq!(
        output,
        format!("-c {} admin token mint --local", config.display())
    );
}

#[test]
fn gives_up_after_the_last_attempt_with_its_error() {
    let dir = tempfile::tempdir().unwrap();
    let binary = FakeBvc::script(dir.path(), r#"echo "database is locked" >&2; exit 1"#);

    let error = FakeBvc::minter(binary, dir.path(), 3, Duration::from_millis(1))
        .mint()
        .unwrap_err();

    assert!(error.to_string().contains("database is locked"));
}

#[test]
fn empty_output_is_not_a_token() {
    let dir = tempfile::tempdir().unwrap();
    let binary = FakeBvc::script(dir.path(), "exit 0");

    assert!(
        FakeBvc::minter(binary, dir.path(), 2, Duration::ZERO)
            .mint()
            .is_err()
    );
}
