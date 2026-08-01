use bvc_server_lib::config::Server;

// A bare IPv6 address must be bracketed before a port is appended, or the result
// parses as neither an address nor a host:port pair and the bind fails at startup
// with a message that names neither cause.
#[test]
fn an_ipv6_listen_address_is_bracketed_before_the_port() {
    let mut config = Server::default();
    config.listen = "::".to_string();

    assert_eq!(config.quic_bind_addr(443), "[::]:443");
}

#[test]
fn an_ipv4_listen_address_is_not_bracketed() {
    let mut config = Server::default();
    config.listen = "0.0.0.0".to_string();

    assert_eq!(config.quic_bind_addr(8443), "0.0.0.0:8443");
}

// An operator who writes the bracketed form must not end up with it doubled.
#[test]
fn an_already_bracketed_listen_address_is_not_bracketed_twice() {
    let mut config = Server::default();
    config.listen = "[::]".to_string();

    assert_eq!(config.quic_bind_addr(443), "[::]:443");
}

// Rocket's `address` key wants a bare IpAddr, so brackets an operator wrote for
// the QUIC form have to come off before the figment sees them.
#[test]
fn the_rocket_address_form_is_unbracketed() {
    let mut config = Server::default();

    config.listen = "0.0.0.0".to_string();
    assert_eq!(config.http_listen_ip(), "0.0.0.0");

    config.listen = "[::1]".to_string();
    assert_eq!(config.http_listen_ip(), "::1");
}

// The HTTP listener cannot follow QUIC onto a wildcard v6 address everywhere. Rocket
// binds through tokio without touching IPV6_V6ONLY, which Windows defaults to
// *enabled*, so `::` there is an IPv6-only listener that refuses every IPv4 client.
// Linux defaults the same flag to disabled, which is why one key served both
// listeners until now.
#[test]
fn a_wildcard_ipv6_listen_downgrades_http_only_where_the_platform_requires_it() {
    let mut config = Server::default();
    config.listen = "::".to_string();

    if cfg!(windows) {
        assert!(config.http_listen_is_downgraded());
        assert_eq!(
            config.http_listen_ip(),
            "0.0.0.0",
            "keeping IPv4 clients working is the safer reading of listen-everywhere              where the wildcard cannot be dual-stack"
        );
    } else {
        assert!(!config.http_listen_is_downgraded());
        assert_eq!(config.http_listen_ip(), "::");
    }
}

// The downgrade decision is (is-wildcard-v6 AND platform-cannot-dual-stack). This
// pins the first half, which is ours; the second half is the platform's and is
// asserted for this platform in the test above.
#[test]
fn only_a_wildcard_ipv6_listen_address_is_a_downgrade_candidate() {
    let mut config = Server::default();

    config.listen = "::".to_string();
    assert!(config.listen_is_wildcard_v6());

    config.listen = "[::]".to_string();
    assert!(config.listen_is_wildcard_v6(), "brackets must not hide the wildcard");

    config.listen = "::1".to_string();
    assert!(!config.listen_is_wildcard_v6());

    config.listen = "0.0.0.0".to_string();
    assert!(!config.listen_is_wildcard_v6());
}

// The downgrade applies only to a *wildcard* v6 address. An operator who names a
// specific v6 address means it, and Rocket can bind that on any platform.
#[test]
fn a_specific_ipv6_listen_address_is_never_downgraded() {
    let mut config = Server::default();
    config.listen = "::1".to_string();

    assert!(!config.http_listen_is_downgraded());
    assert_eq!(config.http_listen_ip(), "::1");
}

// QUIC is dual-stack on every platform regardless of the HTTP downgrade, because
// s2n-quic-platform clears IPV6_V6ONLY on the socket it creates.
#[test]
fn the_quic_bind_address_is_unaffected_by_the_http_downgrade() {
    let mut config = Server::default();
    config.listen = "::".to_string();

    assert_eq!(config.quic_bind_addr(443), "[::]:443");
}

// Dual-stack by default is what carries the fix to installs whose operator changes
// nothing. A v6 wildcard bind serves IPv4 peers as well, because s2n-quic-platform
// clears IPV6_V6ONLY on the socket rather than deferring to the host sysctl.
#[test]
fn the_default_listen_address_is_dual_stack() {
    assert_eq!(Server::default().listen, "::");
}

// The address used when a v6 bind fails, which means the host has no IPv6 stack.
// Without this an upgrade would take those installs offline.
#[test]
fn the_fallback_listen_address_is_the_ipv4_wildcard() {
    assert_eq!(Server::FALLBACK_LISTEN, "0.0.0.0");
}
