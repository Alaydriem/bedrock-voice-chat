use bvc_client_lib::DiscordOAuth;

#[test]
fn authorize_url_has_implicit_grant_params() {
    let url = DiscordOAuth::authorize_url("cid123", "https://example.com/discord/callback", "st8");
    assert!(url.starts_with("https://discord.com/oauth2/authorize?"));
    assert!(url.contains("client_id=cid123"));
    assert!(url.contains("response_type=token"));
    assert!(url.contains("scope=guilds.members.read"));
    assert!(url.contains("redirect_uri=https%3A%2F%2Fexample.com%2Fdiscord%2Fcallback"));
    assert!(url.contains("state=st8"));
}

#[test]
fn parse_fragment_extracts_token_and_state() {
    let frag = "#access_token=abc.def&token_type=Bearer&expires_in=604800&scope=guilds.members.read&state=st8";
    let (token, state) = DiscordOAuth::parse_fragment(frag).expect("parsed");
    assert_eq!(token, "abc.def");
    assert_eq!(state, "st8");
}

#[test]
fn parse_fragment_missing_token_errors() {
    let frag = "#token_type=Bearer&state=st8";
    assert!(DiscordOAuth::parse_fragment(frag).is_err());
}
