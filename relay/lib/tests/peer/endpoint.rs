use bvc_relay::node::{NodeIdentity, PeerTicket};
use bvc_relay::peer::{PeerEndpoint, PeerError};
use iroh::TransportAddr;
use tempfile::TempDir;

// A port nothing else holds. Bound and released rather than picked from a range,
// because a hardcoded one collides with whatever else this machine runs.
fn free_udp_port() -> u16 {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").expect("bind a probe socket");
    socket.local_addr().expect("probe address").port()
}

// How many ports a pinned-port test may lose before it reports a failure.
//
// Neither test can reserve the port it pins: `free_udp_port` returns it to the
// operating system before the bind runs, and from that moment anything on the
// machine can be handed it — including the rest of this run, where every other test
// binds an ephemeral endpoint out of the same pool. The restart path has a second
// window, because `Endpoint::close` waits a bounded 100ms for the socket actor and
// then aborts it: the socket can outlive the call that closed it.
//
// Losing the port is contention rather than a broken contract, so the scenario runs
// again on a fresh one. Bounded, because a machine where no port ever binds is a
// real failure and has to surface as one.
const BIND_ATTEMPTS: usize = 8;

fn ip_ports(addr: &iroh::EndpointAddr) -> Vec<u16> {
    addr.addrs
        .iter()
        .filter_map(|transport| match transport {
            TransportAddr::Ip(socket) => Some(socket.port()),
            _ => None,
        })
        .collect()
}

async fn identity(dir: &TempDir) -> NodeIdentity {
    let path = dir.path().to_str().expect("utf-8 path");
    NodeIdentity::load_or_create(path).expect("identity")
}

// Runs `scenario` on a freshly probed port until one of them survives to be bound.
// Only a bind failure retries; an assertion inside `scenario` panics exactly as it
// would in a test written without this.
async fn on_a_free_port(scenario: impl AsyncFn(u16) -> Result<(), PeerError>) {
    let mut last = None;

    for _ in 0..BIND_ATTEMPTS {
        let port = free_udp_port();
        match scenario(port).await {
            Ok(()) => return,
            Err(error) => last = Some((port, error)),
        }
    }

    panic!("lost the pinned port {BIND_ATTEMPTS} times running, last {last:?}");
}

// The operator-visible contract behind a pinned port: the ticket has to advertise
// it. A ticket carries the addresses a peer will be dialled on, so an endpoint that
// pins its port and then advertises a different one has pinned nothing.
#[tokio::test]
async fn a_pinned_port_is_the_port_the_ticket_advertises() {
    let dir = TempDir::new().expect("temp dir");
    let identity = identity(&dir).await;

    on_a_free_port(async |port| {
        let endpoint = PeerEndpoint::bind_on(&identity, None, Some(port)).await?;

        let addr =
            PeerTicket::parse(&endpoint.ticket().await.expect("ticket")).expect("parse ticket");

        assert!(
            ip_ports(&addr).contains(&port),
            "the ticket advertises {:?}, none of them the pinned {port}",
            ip_ports(&addr)
        );

        Ok(())
    })
    .await;
}

// The reason the port is configurable at all. An operator pastes this server's
// ticket into the far side's config once; a restart that moves the port silently
// invalidates what they pasted, which is what an ephemeral port does on every boot.
#[tokio::test]
async fn a_pinned_port_survives_a_restart_on_the_same_identity() {
    let dir = TempDir::new().expect("temp dir");
    let identity = identity(&dir).await;

    on_a_free_port(async |port| {
        let first = PeerEndpoint::bind_on(&identity, None, Some(port)).await?;
        let before = first.ticket().await.expect("first ticket");

        // Closed rather than dropped. Dropping schedules iroh's teardown without waiting
        // for it, so the socket is still held when the rebind runs — a real restart frees
        // it by exiting the process, and this is the in-process stand-in for that.
        first.endpoint().close().await;
        drop(first);

        let second = PeerEndpoint::bind_on(&identity, None, Some(port)).await?;
        let after = second.ticket().await.expect("second ticket");

        let before = PeerTicket::parse(&before).expect("parse the first ticket");
        let after = PeerTicket::parse(&after).expect("parse the second ticket");

        assert_eq!(before.id, after.id, "the node identity moved");
        assert!(
            ip_ports(&after).contains(&port),
            "the rebind did not land on the pinned port"
        );

        Ok(())
    })
    .await;
}

// Absent a pinned port the endpoint keeps taking whatever the operating system
// hands it, which is what every deployment that never sets one relies on.
#[tokio::test]
async fn no_pinned_port_still_binds() {
    let dir = TempDir::new().expect("temp dir");
    let identity = identity(&dir).await;

    let endpoint = PeerEndpoint::bind_on(&identity, None, None)
        .await
        .expect("bind without a pinned port");

    assert!(
        endpoint.endpoint().bound_sockets().iter().any(|s| s.port() != 0),
        "an unpinned endpoint reported no bound port"
    );
}
