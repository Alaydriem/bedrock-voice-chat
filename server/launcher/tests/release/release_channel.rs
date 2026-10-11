use bvc_launcher::release::ReleaseChannel;

#[test]
fn a_build_without_a_channel_follows_stable() {
    assert_eq!(ReleaseChannel::from_build(None), ReleaseChannel::Stable);
}

#[test]
fn only_beta_selects_the_beta_channel() {
    assert_eq!(ReleaseChannel::from_build(Some("beta")), ReleaseChannel::Beta);
    assert_eq!(ReleaseChannel::from_build(Some("stable")), ReleaseChannel::Stable);
    assert_eq!(ReleaseChannel::from_build(Some("")), ReleaseChannel::Stable);
}

#[test]
fn each_channel_reads_the_manifest_release_yml_writes_for_it() {
    assert_eq!(ReleaseChannel::Stable.manifest_file(), "latest.json");
    assert_eq!(ReleaseChannel::Beta.manifest_file(), "beta.json");
}
