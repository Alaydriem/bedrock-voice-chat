use std::collections::HashSet;

use bvc_client_lib::testkit::PortPool;

// The regression guard for the defect this type exists to remove.
//
// Ports used to come from `TcpListener::bind("127.0.0.1:0")`, whose socket was
// closed before the number was handed to a server that bound it later. The
// operating system hands a just-released ephemeral port straight back out, so two
// test processes racing through that window got the same number and whichever
// server bound second died with `os error 10048`.
#[test]
fn a_port_is_outside_the_range_the_os_hands_out_on_its_own() {
    for _ in 0..16 {
        let port = PortPool::tcp();
        assert!(
            PortPool::RANGE.contains(&port),
            "port {port} is outside the reserved range, so the operating system \
             can hand it to another process on its own"
        );
    }
}

#[test]
fn a_reserved_port_is_actually_bindable() {
    let port = PortPool::tcp();

    std::net::TcpListener::bind(("127.0.0.1", port))
        .unwrap_or_else(|e| panic!("reserved tcp port {port} did not bind: {e}"));
}

#[test]
fn a_reserved_udp_port_is_actually_bindable() {
    let port = PortPool::udp();

    std::net::UdpSocket::bind(("127.0.0.1", port))
        .unwrap_or_else(|e| panic!("reserved udp port {port} did not bind: {e}"));
}

// One scenario reserves several ports and holds them all at once, so handing the
// same number out twice puts two listeners on one port.
#[test]
fn a_port_is_never_handed_out_twice() {
    let mut seen = HashSet::new();

    for _ in 0..64 {
        let port = PortPool::tcp();
        assert!(seen.insert(port), "port {port} was handed out twice");
    }
    for _ in 0..64 {
        let port = PortPool::udp();
        assert!(seen.insert(port), "port {port} was handed out twice");
    }
}

// Scenarios reserve from several tokio worker threads at once.
#[test]
fn concurrent_reservations_are_distinct() {
    let handles: Vec<_> = (0..8)
        .map(|_| std::thread::spawn(|| (0..16).map(|_| PortPool::tcp()).collect::<Vec<_>>()))
        .collect();

    let mut seen = HashSet::new();
    for handle in handles {
        for port in handle.join().expect("reservation thread") {
            assert!(seen.insert(port), "port {port} was handed to two threads");
        }
    }
}

// Every reservation holds a claim other processes can see, which is what stops
// two test processes choosing one port. A reservation without one is invisible.
#[test]
fn a_reservation_leaves_a_claim_other_processes_can_see() {
    let port = PortPool::tcp();

    assert!(
        PortPool::claim_path(port).exists(),
        "port {port} was reserved without a claim, so another test process \
         reserving it would see nothing in its way"
    );
}
