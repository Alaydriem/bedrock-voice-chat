use bvc_relay_service::config::PlayCaEndpoint;

// Each form an operator writes: a single-host loopback, a compose service name, a bridge
// address and an IPv6 address in brackets.
#[test]
fn an_endpoint_accepts_an_ip_a_hostname_or_a_bracketed_ipv6_address() {
    for (value, host, port) in [
        ("127.0.0.1:9000", "127.0.0.1", 9000),
        ("play-ca:9000", "play-ca", 9000),
        ("172.30.0.10:9443", "172.30.0.10", 9443),
        ("[::1]:9000", "::1", 9000),
    ] {
        let endpoint: PlayCaEndpoint = value.parse().unwrap_or_else(|e| panic!("{value}: {e}"));

        assert_eq!(endpoint.host(), host, "{value}");
        assert_eq!(endpoint.port(), port, "{value}");
    }
}

// An IPv6 address without brackets is ambiguous: its last group cannot be told apart from
// the port. The rest are typos that would otherwise surface only at the first issuance.
#[test]
fn an_endpoint_without_a_usable_host_and_port_is_refused() {
    for value in [
        "play-ca",
        "play-ca:",
        ":9000",
        "play-ca:0",
        "play-ca:70000",
        "play-ca:port",
        "::1:9000",
        "[::1:9000",
        "play ca:9000",
    ] {
        assert!(value.parse::<PlayCaEndpoint>().is_err(), "{value}");
    }
}

// The bracketed form is written back with its brackets, so a value the registry reports in
// a log or error can be pasted into the config again.
#[test]
fn an_ipv6_endpoint_is_displayed_with_its_brackets() {
    let endpoint: PlayCaEndpoint = "[fd00::10]:9000".parse().expect("parses");

    assert_eq!(endpoint.to_string(), "[fd00::10]:9000");
}
