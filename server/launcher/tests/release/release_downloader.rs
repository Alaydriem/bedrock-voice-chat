use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use bvc_launcher::LauncherPaths;
use bvc_launcher::release::{ReleaseAssets, ReleaseDownloader, ReleaseSources};

use crate::support::{HttpFixture, HttpReply};

struct Release;

impl Release {
    const TAG: &'static str = "v2.0.0";

    fn sources(http: &HttpFixture) -> ReleaseSources {
        ReleaseSources::new(
            vec![http.url("/latest.json"), http.url("/beta.json")],
            http.url("/releases/download"),
            Duration::from_secs(2),
        )
    }

    fn manifest(http: &HttpFixture) -> HttpReply {
        let url = http.url(&format!("/releases/download/{}/bvc-client.AppImage", Self::TAG));
        HttpReply::Body(
            format!(
                r#"{{"pub_date":"2026-10-01T00:00:00.000Z","platforms":{{"linux-x86_64":{{"url":"{url}"}}}}}}"#
            )
            .into_bytes(),
        )
    }

    // The fixture path for an asset URL of this release.
    fn path_of(http: &HttpFixture, url: &str) -> String {
        url.trim_start_matches(&http.url("")).to_string()
    }

    fn server_path(http: &HttpFixture) -> String {
        let assets = ReleaseAssets::new(http.url("/releases/download"), Self::TAG.to_string());
        Self::path_of(http, &assets.server_url(std::env::consts::ARCH).unwrap())
    }

    fn pack_path(http: &HttpFixture) -> String {
        let assets = ReleaseAssets::new(http.url("/releases/download"), Self::TAG.to_string());
        Self::path_of(http, &assets.pack_url())
    }

    fn install(paths: &LauncherPaths, server: &str, pack: &str, version: &str) {
        fs::create_dir_all(paths.bvc_binary().parent().unwrap()).unwrap();
        fs::create_dir_all(paths.pack_file().parent().unwrap()).unwrap();
        fs::write(paths.bvc_binary(), server).unwrap();
        fs::write(paths.pack_file(), pack).unwrap();
        fs::write(paths.version_file(), version).unwrap();
    }

    fn has_temp_files(dir: &Path) -> bool {
        fs::read_dir(dir)
            .unwrap()
            .any(|e| e.unwrap().file_name().to_string_lossy().ends_with(".tmp"))
    }
}

#[test]
fn a_new_release_installs_the_server_the_pack_and_its_version() {
    let http = HttpFixture::start();
    http.route("/beta.json", Release::manifest(&http));
    http.route(&Release::server_path(&http), HttpReply::Body(b"server".to_vec()));
    http.route(&Release::pack_path(&http), HttpReply::Body(b"pack".to_vec()));
    let root = tempfile::tempdir().unwrap();
    let paths = LauncherPaths::new(root.path().to_path_buf());

    ReleaseDownloader::new(paths.clone(), Release::sources(&http))
        .unwrap()
        .update()
        .unwrap();

    assert_eq!(fs::read(paths.bvc_binary()).unwrap(), b"server");
    assert_eq!(fs::read(paths.pack_file()).unwrap(), b"pack");
    assert_eq!(fs::read_to_string(paths.version_file()).unwrap(), Release::TAG);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(paths.bvc_binary()).unwrap().permissions().mode();
        assert_ne!(mode & 0o111, 0);
    }
}

#[test]
fn a_failed_pack_download_leaves_the_installed_release_untouched() {
    let http = HttpFixture::start();
    http.route("/beta.json", Release::manifest(&http));
    http.route(&Release::server_path(&http), HttpReply::Body(b"new server".to_vec()));
    http.route(&Release::pack_path(&http), HttpReply::Status(500));
    let root = tempfile::tempdir().unwrap();
    let paths = LauncherPaths::new(root.path().to_path_buf());
    Release::install(&paths, "old server", "old pack", "v1.0.0");

    let result = ReleaseDownloader::new(paths.clone(), Release::sources(&http))
        .unwrap()
        .update();

    assert!(result.is_err());
    assert_eq!(fs::read_to_string(paths.bvc_binary()).unwrap(), "old server");
    assert_eq!(fs::read_to_string(paths.pack_file()).unwrap(), "old pack");
    assert_eq!(fs::read_to_string(paths.version_file()).unwrap(), "v1.0.0");
    assert!(!Release::has_temp_files(paths.bvc_binary().parent().unwrap()));
    assert!(!Release::has_temp_files(paths.pack_file().parent().unwrap()));
}

#[test]
fn unusable_manifests_change_no_file() {
    let http = HttpFixture::start();
    http.route("/latest.json", HttpReply::Body(b"<html>404</html>".to_vec()));
    http.route("/beta.json", HttpReply::Status(404));
    let root = tempfile::tempdir().unwrap();
    let paths = LauncherPaths::new(root.path().to_path_buf());

    let result = ReleaseDownloader::new(paths.clone(), Release::sources(&http))
        .unwrap()
        .update();

    assert!(result.is_err());
    assert!(!paths.bvc_dir().exists());
}

#[test]
fn an_installed_release_is_not_downloaded_again() {
    let http = HttpFixture::start();
    http.route("/beta.json", Release::manifest(&http));
    http.route(&Release::server_path(&http), HttpReply::Status(500));
    let root = tempfile::tempdir().unwrap();
    let paths = LauncherPaths::new(root.path().to_path_buf());
    Release::install(&paths, "current server", "current pack", Release::TAG);

    ReleaseDownloader::new(paths.clone(), Release::sources(&http))
        .unwrap()
        .update()
        .unwrap();

    assert_eq!(fs::read_to_string(paths.bvc_binary()).unwrap(), "current server");
}

#[test]
fn a_stalled_manifest_server_does_not_hold_the_start() {
    let http = HttpFixture::start();
    http.route("/latest.json", HttpReply::Stall);
    http.route("/beta.json", Release::manifest(&http));
    let root = tempfile::tempdir().unwrap();
    let paths = LauncherPaths::new(root.path().to_path_buf());
    Release::install(&paths, "current server", "current pack", Release::TAG);
    let started = Instant::now();

    ReleaseDownloader::new(paths.clone(), Release::sources(&http))
        .unwrap()
        .update()
        .unwrap();

    assert!(started.elapsed() < Duration::from_secs(10));
}
