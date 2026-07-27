use bvc_server_lib::config::Server;

#[test]
fn unset_list_advertises_only_the_bind_port() {
    let mut config = Server::default();
    config.quic_port = 8443;

    assert_eq!(
        config.quic_ports(),
        vec![8443u32],
        "an operator who never opted in must see exactly today's behaviour"
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
