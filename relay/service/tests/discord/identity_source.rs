use bvc_relay_service::discord::{DiscordOAuthClient, IdentitySource};

// The bot token reads roles, so the user's own token is needed only to learn which
// account is at the browser. Asking for more than `identify` would collect data this
// registry has no use for.
#[test]
fn the_oauth_scope_is_identify_and_nothing_more() {
    assert_eq!(DiscordOAuthClient::SCOPE, "identify");
}

// The authorize URL carries the state through Discord unchanged. It is the other half
// of the CSRF check, and a URL that dropped it would make the callback's comparison
// always fail.
#[test]
fn the_authorize_url_carries_the_state_and_the_redirect() {
    let url = DiscordOAuthClient::authorize_url(
        "client-123",
        "https://registry.example/oauth/callback",
        "state-abc",
    );

    assert!(url.starts_with("https://discord.com/oauth2/authorize?"));
    assert!(url.contains("state=state-abc"));
    assert!(url.contains("response_type=code"));
    assert!(url.contains("scope=identify"));
    assert!(url.contains("client_id=client-123"));
}

// The authorization-code flow, not implicit. Implicit returns the token in the URL
// fragment, which a server-side callback never receives — this registry holds a
// client secret precisely so it does not need that flow.
#[test]
fn the_authorize_url_never_requests_an_implicit_token() {
    let url = DiscordOAuthClient::authorize_url("c", "https://registry.example/cb", "s");

    assert!(!url.contains("response_type=token"));
}

#[tokio::test]
async fn a_fixed_identity_source_answers_without_discord() {
    let source = IdentitySource::Fixed("member-1".to_string());

    assert_eq!(
        source.identify("any-code").await.expect("identifies"),
        "member-1"
    );
}
