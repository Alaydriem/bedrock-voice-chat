use bvc_server_lib::config::Server;

// Both public ways in, without the operator configuring anything: a client whose
// network drops UDP/443 has a second port to try. The list is a statement about
// public reachability rather than about what this process binds, so moving the bind
// port does not move it — an operator who does that names the list too.
#[test]
fn an_untouched_list_advertises_both_default_ports() {
    let config = Server::default();

    assert_eq!(config.quic_ports(), vec![28280]);
}

#[test]
fn a_named_list_replaces_the_default_and_the_bind_port() {
    let mut config = Server::default();
    config.quic_port = 8443;
    config.advertised_quic_ports = vec![9443];

    assert_eq!(
        config.quic_ports(),
        vec![9443u32],
        "a list in config.hcl overrides everything"
    );
}

#[test]
fn explicit_list_replaces_the_bind_port_and_keeps_order() {
    let mut config = Server::default();
    config.quic_port = 8443;
    config.advertised_quic_ports = vec![443, 8443];

    assert_eq!(
        config.quic_ports(),
        vec![443u32, 8443],
        "the bind port is not implicitly prepended; the list is what clients are told"
    );
}

#[test]
fn advertised_list_may_exclude_the_bind_port() {
    let mut config = Server::default();
    config.quic_port = 8443;
    config.advertised_quic_ports = vec![443];

    assert_eq!(
        config.quic_ports(),
        vec![443u32],
        "behind a fronting proxy the bind port is internal and may be unreachable from outside"
    );
}
