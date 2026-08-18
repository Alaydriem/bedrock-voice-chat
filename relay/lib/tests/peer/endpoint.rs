use bvc_relay::node::{NodeIdentity, PeerTicket};
use bvc_relay::peer::PeerEndpoint;
use iroh::TransportAddr;
use tempfile::TempDir;

// A port nothing else holds. Bound and released rather than picked from a range,
// because a hardcoded one collides with whatever else this machine runs.
fn free_udp_port() -> u16 {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").expect("bind a probe socket");
    socket.local_addr().expect("probe address").port()
}

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

// The operator-visible contract behind a pinned port: the ticket has to advertise
// it. A ticket carries the addresses a peer will be dialled on, so an endpoint that
// pins its port and then advertises a different one has pinned nothing.
#[tokio::test]
async fn a_pinned_port_is_the_port_the_ticket_advertises() {
    let dir = TempDir::new().expect("temp dir");
    let identity = identity(&dir).await;
    let port = free_udp_port();

    let endpoint = PeerEndpoint::bind_on(&identity, None, Some(port))
        .await
        .expect("bind on a pinned port");

    let addr = PeerTicket::parse(&endpoint.ticket().await.expect("ticket")).expect("parse ticket");

    assert!(
        ip_ports(&addr).contains(&port),
        "the ticket advertises {:?}, none of them the pinned {port}",
        ip_ports(&addr)
    );
}

// The reason the port is configurable at all. An operator pastes this server's
// ticket into the far side's config once; a restart that moves the port silently
// invalidates what they pasted, which is what an ephemeral port does on every boot.
#[tokio::test]
async fn a_pinned_port_survives_a_restart_on_the_same_identity() {
    let dir = TempDir::new().expect("temp dir");
    let identity = identity(&dir).await;
    let port = free_udp_port();

    let first = PeerEndpoint::bind_on(&identity, None, Some(port))
        .await
        .expect("first bind");
    let before = first.ticket().await.expect("first ticket");

    // Closed rather than dropped. Dropping schedules iroh's teardown without waiting
    // for it, so the socket is still held when the rebind runs — a real restart frees
    // it by exiting the process, and this is the in-process stand-in for that.
    first.endpoint().close().await;
    drop(first);

    let second = PeerEndpoint::bind_on(&identity, None, Some(port))
        .await
        .expect("rebind on the same port");
    let after = second.ticket().await.expect("second ticket");

    let before = PeerTicket::parse(&before).expect("parse the first ticket");
    let after = PeerTicket::parse(&after).expect("parse the second ticket");

    assert_eq!(before.id, after.id, "the node identity moved");
    assert!(
        ip_ports(&after).contains(&port),
        "the rebind did not land on the pinned port"
    );
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
