use bvc_client_lib::ServerEndpoint;

// The reported defect: AUTH_ENDPOINT carries a leading slash and the old join added its own, so
// every Minecraft auth request went to //api/auth/minecraft.
#[test]
fn a_leading_slash_on_the_path_does_not_double_the_separator() {
    assert_eq!(
        ServerEndpoint::join("https://example.invalid", "/api/auth/minecraft"),
        "https://example.invalid/api/auth/minecraft"
    );
}

// The other call site passes a path with no leading slash. Both must produce the same URL, or the
// join has simply moved the asymmetry rather than removed it.
#[test]
fn a_path_without_a_leading_slash_produces_the_same_url() {
    assert_eq!(
        ServerEndpoint::join("https://example.invalid", "api/auth/code"),
        "https://example.invalid/api/auth/code"
    );
}

// A server URL saved with a trailing slash is normal - it is whatever the user typed.
#[test]
fn a_trailing_slash_on_the_server_is_absorbed() {
    assert_eq!(
        ServerEndpoint::join("https://example.invalid/", "/api/auth/minecraft"),
        "https://example.invalid/api/auth/minecraft"
    );
    assert_eq!(
        ServerEndpoint::join("https://example.invalid/", "api/auth/minecraft"),
        "https://example.invalid/api/auth/minecraft"
    );
}

// Several trailing slashes should not survive either. A user pasting a URL is the source.
#[test]
fn repeated_separators_collapse() {
    assert_eq!(
        ServerEndpoint::join("https://example.invalid///", "///api/auth/minecraft"),
        "https://example.invalid/api/auth/minecraft"
    );
}

// A port must not be mistaken for a path separator.
#[test]
fn a_port_is_preserved() {
    assert_eq!(
        ServerEndpoint::join("https://example.invalid:8443", "/api/config"),
        "https://example.invalid:8443/api/config"
    );
}
