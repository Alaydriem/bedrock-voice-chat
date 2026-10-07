use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use bvc_launcher::LauncherPaths;
use bvc_launcher::release::{ReleaseAssets, ReleaseChannel, ReleaseDownloader, ReleaseSources};

use crate::support::{HttpFixture, HttpReply};

struct Release;

impl Release {
    const TAG: &'static str = "v2.0.0";
    const BETA_TAG: &'static str = "v2.1.0-beta.1";

    fn sources(http: &HttpFixture, channel: ReleaseChannel) -> ReleaseSources {
        ReleaseSources::new(
            http.url("/updater"),
            http.url("/releases/download"),
            Duration::from_secs(2),
            channel,
        )
    }

    fn manifest(http: &HttpFixture, tag: &str, pub_date: &str) -> HttpReply {
        let url = http.url(&format!("/releases/download/{tag}/bvc-client.AppImage"));
        HttpReply::Body(
            format!(
                r#"{{"pub_date":"{pub_date}","platforms":{{"linux-x86_64":{{"url":"{url}"}}}}}}"#
            )
            .into_bytes(),
        )
    }

    // The fixture path for an asset URL of a release.
    fn path_of(http: &HttpFixture, url: &str) -> String {
        url.trim_start_matches(&http.url("")).to_string()
    }

    fn server_path(http: &HttpFixture, tag: &str) -> String {
        let assets = ReleaseAssets::new(http.url("/releases/download"), tag.to_string());
        Self::path_of(http, &assets.server_url(std::env::consts::ARCH).unwrap())
    }

    fn pack_path(http: &HttpFixture, tag: &str) -> String {
        let assets = ReleaseAssets::new(http.url("/releases/download"), tag.to_string());
        Self::path_of(http, &assets.pack_url())
    }

    // Publishes a stable and a beta release; the one not named by `newer` is a month older.
    fn publish_both(http: &HttpFixture, newer: ReleaseChannel) {
        let (stable_date, beta_date) = match newer {
            ReleaseChannel::Stable => ("2026-11-01T00:00:00.000Z", "2026-10-01T00:00:00.000Z"),
            ReleaseChannel::Beta => ("2026-10-01T00:00:00.000Z", "2026-11-01T00:00:00.000Z"),
        };
        http.route("/updater/latest.json", Self::manifest(http, Self::TAG, stable_date));
        http.route("/updater/beta.json", Self::manifest(http, Self::BETA_TAG, beta_date));
        for tag in [Self::TAG, Self::BETA_TAG] {
            http.route(&Self::server_path(http, tag), HttpReply::Body(tag.as_bytes().to_vec()));
            http.route(&Self::pack_path(http, tag), HttpReply::Body(b"pack".to_vec()));
        }
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
    http.route(
        "/updater/latest.json",
        Release::manifest(&http, Release::TAG, "2026-10-01T00:00:00.000Z"),
    );
    http.route(&Release::server_path(&http, Release::TAG), HttpReply::Body(b"server".to_vec()));
    http.route(&Release::pack_path(&http, Release::TAG), HttpReply::Body(b"pack".to_vec()));
    let root = tempfile::tempdir().unwrap();
    let paths = LauncherPaths::new(root.path().to_path_buf());

    ReleaseDownloader::new(paths.clone(), Release::sources(&http, ReleaseChannel::Stable))
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
fn a_stable_launcher_does_not_take_a_newer_beta() {
    let http = HttpFixture::start();
    Release::publish_both(&http, ReleaseChannel::Beta);
    let root = tempfile::tempdir().unwrap();
    let paths = LauncherPaths::new(root.path().to_path_buf());

    ReleaseDownloader::new(paths.clone(), Release::sources(&http, ReleaseChannel::Stable))
        .unwrap()
        .update()
        .unwrap();

    assert_eq!(fs::read_to_string(paths.version_file()).unwrap(), Release::TAG);
    assert_eq!(fs::read_to_string(paths.bvc_binary()).unwrap(), Release::TAG);
}

#[test]
fn a_beta_launcher_does_not_take_a_newer_stable_release() {
    let http = HttpFixture::start();
    Release::publish_both(&http, ReleaseChannel::Stable);
    let root = tempfile::tempdir().unwrap();
    let paths = LauncherPaths::new(root.path().to_path_buf());

    ReleaseDownloader::new(paths.clone(), Release::sources(&http, ReleaseChannel::Beta))
        .unwrap()
        .update()
        .unwrap();

    assert_eq!(fs::read_to_string(paths.version_file()).unwrap(), Release::BETA_TAG);
    assert_eq!(fs::read_to_string(paths.bvc_binary()).unwrap(), Release::BETA_TAG);
}

#[test]
fn a_failed_pack_download_leaves_the_installed_release_untouched() {
    let http = HttpFixture::start();
    http.route(
        "/updater/latest.json",
        Release::manifest(&http, Release::TAG, "2026-10-01T00:00:00.000Z"),
    );
    http.route(
        &Release::server_path(&http, Release::TAG),
        HttpReply::Body(b"new server".to_vec()),
    );
    http.route(&Release::pack_path(&http, Release::TAG), HttpReply::Status(500));
    let root = tempfile::tempdir().unwrap();
    let paths = LauncherPaths::new(root.path().to_path_buf());
    Release::install(&paths, "old server", "old pack", "v1.0.0");

    let result =
        ReleaseDownloader::new(paths.clone(), Release::sources(&http, ReleaseChannel::Stable))
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
fn an_unusable_manifest_changes_no_file() {
    let http = HttpFixture::start();
    http.route("/updater/latest.json", HttpReply::Body(b"<html>404</html>".to_vec()));
    let root = tempfile::tempdir().unwrap();
    let paths = LauncherPaths::new(root.path().to_path_buf());

    let result =
        ReleaseDownloader::new(paths.clone(), Release::sources(&http, ReleaseChannel::Stable))
            .unwrap()
            .update();

    assert!(result.is_err());
    assert!(!paths.bvc_dir().exists());
}

#[test]
fn an_installed_release_is_not_downloaded_again() {
    let http = HttpFixture::start();
    http.route(
        "/updater/latest.json",
        Release::manifest(&http, Release::TAG, "2026-10-01T00:00:00.000Z"),
    );
    http.route(&Release::server_path(&http, Release::TAG), HttpReply::Status(500));
    let root = tempfile::tempdir().unwrap();
    let paths = LauncherPaths::new(root.path().to_path_buf());
    Release::install(&paths, "current server", "current pack", Release::TAG);

    ReleaseDownloader::new(paths.clone(), Release::sources(&http, ReleaseChannel::Stable))
        .unwrap()
        .update()
        .unwrap();

    assert_eq!(fs::read_to_string(paths.bvc_binary()).unwrap(), "current server");
}

#[test]
fn a_stalled_manifest_server_does_not_hold_the_start() {
    let http = HttpFixture::start();
    http.route("/updater/latest.json", HttpReply::Stall);
    let root = tempfile::tempdir().unwrap();
    let paths = LauncherPaths::new(root.path().to_path_buf());
    Release::install(&paths, "current server", "current pack", Release::TAG);
    let started = Instant::now();

    let result =
        ReleaseDownloader::new(paths.clone(), Release::sources(&http, ReleaseChannel::Stable))
            .unwrap()
            .update();

    assert!(result.is_err());
    assert!(started.elapsed() < Duration::from_secs(10));
    assert_eq!(fs::read_to_string(paths.bvc_binary()).unwrap(), "current server");
}
