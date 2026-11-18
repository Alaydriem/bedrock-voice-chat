use std::net::TcpListener;
use std::time::{Duration, Instant};

use bvc_client_lib::network::DefaultPortProbe;

const HOST: &str = "127.0.0.1";

struct Ports;

impl Ports {
    // A port with a listener behind it. The listener must outlive the probe: a connect
    // completes from the backlog without anything calling accept.
    fn open() -> (TcpListener, u16) {
        let listener = TcpListener::bind((HOST, 0)).expect("a loopback listener");
        let port = listener.local_addr().expect("a bound address").port();
        (listener, port)
    }

    // A port nothing listens on, so a connect is refused at once.
    fn closed() -> u16 {
        let (listener, port) = Self::open();
        drop(listener);
        port
    }

    fn probe(ports: Vec<u16>) -> DefaultPortProbe {
        DefaultPortProbe::new(ports, Duration::from_millis(1500))
    }
}

#[tokio::test]
async fn the_first_port_wins_when_both_accept() {
    let (_first, first) = Ports::open();
    let (_second, second) = Ports::open();

    let picked = Ports::probe(vec![first, second]).pick(HOST).await;

    assert_eq!(picked, Some(first));
}

#[tokio::test]
async fn a_later_port_is_used_when_the_first_refuses() {
    let refused = Ports::closed();
    let (_open, open) = Ports::open();

    let picked = Ports::probe(vec![refused, open]).pick(HOST).await;

    assert_eq!(picked, Some(open));
}

#[tokio::test]
async fn no_port_is_picked_when_nothing_accepts() {
    let picked = Ports::probe(vec![Ports::closed(), Ports::closed()])
        .pick(HOST)
        .await;

    assert_eq!(picked, None);
}

#[tokio::test]
async fn an_empty_candidate_list_picks_nothing() {
    assert_eq!(Ports::probe(Vec::new()).pick(HOST).await, None);
}

#[tokio::test]
async fn a_host_that_never_answers_is_bounded_by_the_budget() {
    // TEST-NET-1 (RFC 5737): never routed, so a connect either hangs or fails fast.
    let probe = DefaultPortProbe::new(vec![443, 28280], Duration::from_millis(200));

    let started = Instant::now();
    let picked = probe.pick("192.0.2.1").await;

    assert_eq!(picked, None);
    assert!(started.elapsed() < Duration::from_secs(1));
}
