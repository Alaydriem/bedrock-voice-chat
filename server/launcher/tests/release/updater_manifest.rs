use bvc_launcher::release::UpdaterManifest;

struct Manifests;

impl Manifests {
    fn with(pub_date: &str, url: &str) -> String {
        format!(
            r#"{{"version":"1.0.521","notes":"Release","pub_date":"{pub_date}",
            "platforms":{{"linux-x86_64":{{"signature":"sig","url":"{url}"}}}}}}"#
        )
    }
}

#[test]
fn the_tag_is_read_from_a_platform_download_url() {
    let manifest = UpdaterManifest::parse(&Manifests::with(
        "2026-09-17T01:40:15.722Z",
        "https://github.com/Alaydriem/bedrock-voice-chat/releases/download/v1.0.0-beta.21/bvc-client-linux-x86_64.AppImage",
    ))
    .unwrap();

    assert_eq!(manifest.tag().as_deref(), Some("v1.0.0-beta.21"));
}

#[test]
fn a_manifest_without_a_download_url_has_no_tag() {
    let manifest = UpdaterManifest::parse(&Manifests::with(
        "2026-09-17T01:40:15.722Z",
        "https://example.com/not-a-release.AppImage",
    ))
    .unwrap();

    assert_eq!(manifest.tag(), None);
}

#[test]
fn a_manifest_without_platforms_has_no_tag() {
    let manifest =
        UpdaterManifest::parse(r#"{"pub_date":"2026-09-17T01:40:15.722Z","platforms":{}}"#)
            .unwrap();

    assert_eq!(manifest.tag(), None);
}

#[test]
fn the_newer_pub_date_wins() {
    let beta = UpdaterManifest::parse(&Manifests::with(
        "2026-09-17T01:40:15.722Z",
        "https://github.com/Alaydriem/bedrock-voice-chat/releases/download/v1.0.0-beta.21/a",
    ))
    .unwrap();
    let stable = UpdaterManifest::parse(&Manifests::with(
        "2026-11-02T08:00:00.000Z",
        "https://github.com/Alaydriem/bedrock-voice-chat/releases/download/v1.0.0/a",
    ))
    .unwrap();

    let newest = UpdaterManifest::newest(vec![beta, stable]).unwrap();

    assert_eq!(newest.tag().as_deref(), Some("v1.0.0"));
}
