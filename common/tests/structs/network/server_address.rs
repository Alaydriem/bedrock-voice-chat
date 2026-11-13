use common::structs::network::ServerAddress;

#[test]
fn identity_is_the_stored_string_unchanged() {
    for input in [
        "https://Brave.Example.com/",
        " https://a.example.com",
        "a.example.com:30001",
    ] {
        let address = ServerAddress::parse(input).unwrap();
        assert_eq!(address.identity(), input);
    }
}

#[test]
fn a_bare_play_name_parses() {
    let address = ServerAddress::parse("https://Brave-Otter.play.bedrockvc.stream").unwrap();
    assert_eq!(address.host(), "brave-otter.play.bedrockvc.stream");
    assert_eq!(address.explicit_port(), None);
    assert!(address.is_play_name());
    assert!(!address.is_ip_literal());
}

#[test]
fn an_explicit_port_and_trailing_slash_parse() {
    let address = ServerAddress::parse("brave.example.com:30001/").unwrap();
    assert_eq!(address.host(), "brave.example.com");
    assert_eq!(address.explicit_port(), Some(30001));
}

#[test]
fn a_lookalike_suffix_is_not_a_play_name() {
    let address = ServerAddress::parse("https://evilplay.bedrockvc.stream").unwrap();
    assert!(!address.is_play_name());
}

#[test]
fn an_ip_literal_is_detected() {
    let address = ServerAddress::parse("https://203.0.113.9").unwrap();
    assert!(address.is_ip_literal());
    assert_eq!(address.explicit_port(), None);
}

#[test]
fn an_ipv6_literal_is_bracketed_in_urls_and_bare_for_tls() {
    let address = ServerAddress::parse("https://[2001:db8::1]:8443").unwrap();
    assert_eq!(address.host(), "2001:db8::1");
    assert_eq!(address.url_host(), "[2001:db8::1]");
    assert_eq!(address.explicit_port(), Some(8443));
}

#[test]
fn an_explicit_default_port_is_still_explicit() {
    for (input, expected) in [
        ("https://brave.example.com:443", Some(443)),
        ("brave.example.com:443", Some(443)),
        ("http://brave.example.com:80", Some(80)),
        ("https://[2001:db8::1]:443", Some(443)),
        ("https://brave.example.com", None),
        ("https://[2001:db8::1]", None),
    ] {
        let address = ServerAddress::parse(input).unwrap();
        assert_eq!(address.explicit_port(), expected, "{input:?}");
    }
}

#[test]
fn a_port_of_zero_is_refused() {
    assert!(ServerAddress::parse("brave.example.com:0").is_err());
}

#[test]
fn an_address_without_a_host_is_refused() {
    for input in ["", "https://", "https://:443"] {
        assert!(ServerAddress::parse(input).is_err(), "{input:?}");
    }
}
