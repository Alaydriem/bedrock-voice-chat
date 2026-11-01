//! Which peers may use the plain-HTTP branch of the demultiplexer.
//!
//! A peer that is not on this machine cannot be produced on a test machine, so this table
//! is what pins the rule that keeps the branch local.

use std::net::IpAddr;

use bvc_server_lib::demux::PeerClass;

#[test]
fn only_loopback_addresses_are_loopback() {
    let cases: [(&str, bool); 9] = [
        ("127.0.0.1", true),
        ("127.255.0.9", true),
        ("::1", true),
        ("::ffff:127.0.0.1", true),
        ("10.0.0.1", false),
        ("::ffff:10.0.0.1", false),
        ("::2", false),
        ("128.0.0.1", false),
        ("0.0.0.0", false),
    ];

    for (address, expected) in cases {
        let ip: IpAddr = address.parse().expect("test address parses");
        assert_eq!(
            PeerClass::is_loopback(ip),
            expected,
            "{address} must be classed loopback = {expected}"
        );
    }
}
