use bvc_launcher::config::{BvcConfig, PlayDefaults};

#[test]
fn rendered_defaults_parse_as_play_mode_with_loopback() {
    let config = BvcConfig::from_hcl_str(&PlayDefaults::render(8443, None)).unwrap();

    assert!(config.is_play_mode());
    assert!(!config.names_no_certificate_source(None));
    assert_eq!(config.server_url().as_deref(), Some("http://127.0.0.1:8443"));
}

#[test]
fn rendered_defaults_turn_the_loopback_listener_on() {
    let config = BvcConfig::from_hcl_str(&PlayDefaults::render(8443, None)).unwrap();

    assert!(config.server.mod_loopback);
}

#[test]
fn an_unusable_port_variable_is_reported_with_the_port_used() {
    for raw in ["2560O", "0", "70000", "-1"] {
        let warning = PlayDefaults::port_warning(Some(raw)).expect(raw);
        assert!(warning.contains("BVC_PORT"), "{warning}");
        assert!(warning.contains(&PlayDefaults::DEFAULT_PORT.to_string()), "{warning}");
    }
}

#[test]
fn an_unset_blank_or_valid_port_variable_is_quiet() {
    for raw in [None, Some(""), Some("   "), Some("25600")] {
        assert_eq!(PlayDefaults::port_warning(raw), None, "{raw:?}");
    }
}

#[test]
fn an_address_hint_is_written_into_the_play_block() {
    let config =
        BvcConfig::from_hcl_str(&PlayDefaults::render(443, Some("203.0.113.7"))).unwrap();

    let play = config.server.tls.play.expect("play block");
    assert_eq!(play.address.as_deref(), Some("203.0.113.7"));
}

#[test]
fn no_address_leaves_the_play_block_without_one() {
    let config = BvcConfig::from_hcl_str(&PlayDefaults::render(443, None)).unwrap();

    assert_eq!(config.server.tls.play.expect("play block").address, None);
}

#[test]
fn an_address_cannot_break_out_of_its_string() {
    let hostile = "1.2.3.4\"\n  port = 1\n#${env.X}";
    let config = BvcConfig::from_hcl_str(&PlayDefaults::render(8443, Some(hostile))).unwrap();

    assert_eq!(config.server.port, 8443);
    assert_eq!(config.server.tls.play.unwrap().address.as_deref(), Some(hostile));
}

#[test]
fn the_port_and_address_come_from_the_panel_variables() {
    let (port, address) = PlayDefaults::from_values(Some("25600"), Some(" 198.51.100.4 "));

    assert_eq!(port, 25600);
    assert_eq!(address.as_deref(), Some("198.51.100.4"));
}

#[test]
fn a_missing_or_unusable_port_falls_back_to_the_server_default() {
    for port in [None, Some(""), Some("http"), Some("0"), Some("70000")] {
        let (resolved, _) = PlayDefaults::from_values(port, None);
        assert_eq!(resolved, PlayDefaults::DEFAULT_PORT, "port {port:?}");
    }
}

#[test]
fn a_blank_address_is_no_address() {
    let (_, address) = PlayDefaults::from_values(None, Some("   "));

    assert_eq!(address, None);
}
