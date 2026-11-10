use bvc_server_lib::services::play::PlayPreflightReport;

#[test]
fn a_declared_address_that_is_not_public_asks_the_operator_to_change_it() {
    let message = PlayPreflightReport::not_public_message(true);

    assert!(message.contains("Remove server.tls.play.address"), "{message}");
}

// With nothing declared there is nothing to remove, and the message names the network.
#[test]
fn an_observed_address_that_is_not_public_names_the_network_and_asks_for_the_setting() {
    let message = PlayPreflightReport::not_public_message(false);

    assert!(!message.contains("Remove"), "{message}");
    assert!(message.contains("private address"), "{message}");
    assert!(message.contains("Set server.tls.play.address to this server's public IP"), "{message}");
}

#[test]
fn an_unreachable_server_is_told_about_cgnat_behind_a_router_and_a_different_outbound_address() {
    for message in [PlayPreflightReport::TIMED_OUT, PlayPreflightReport::REFUSED] {
        assert!(message.contains("100.64.0.0/10"), "{message}");
        assert!(message.contains("set server.tls.play.address to it"), "{message}");
    }
}
