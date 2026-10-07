use bvc_launcher::release::ReleaseAssets;

struct Github;

impl Github {
    const BASE: &'static str = "https://github.com/Alaydriem/bedrock-voice-chat/releases/download";

    fn assets(tag: &str) -> ReleaseAssets {
        ReleaseAssets::new(Self::BASE.to_string(), tag.to_string())
    }
}

#[test]
fn server_asset_follows_the_linux_release_names() {
    let assets = Github::assets("v1.0.0-beta.21");

    assert_eq!(
        assets.server_url("x86_64").as_deref(),
        Some("https://github.com/Alaydriem/bedrock-voice-chat/releases/download/v1.0.0-beta.21/bvc-server-linux-x64")
    );
    assert_eq!(
        assets.server_url("aarch64").as_deref(),
        Some("https://github.com/Alaydriem/bedrock-voice-chat/releases/download/v1.0.0-beta.21/bvc-server-linux-arm64")
    );
}

#[test]
fn an_unsupported_architecture_has_no_server_asset() {
    assert_eq!(Github::assets("v1.0.0").server_url("riscv64"), None);
}

#[test]
fn the_pack_comes_from_the_matching_mods_release() {
    assert_eq!(
        Github::assets("v1.0.0-beta.21").pack_url(),
        "https://github.com/Alaydriem/bedrock-voice-chat/releases/download/mods-v1.0.0-beta.21/bvc-bds-pack-1.0.0-beta.21.mcaddon"
    );
}
