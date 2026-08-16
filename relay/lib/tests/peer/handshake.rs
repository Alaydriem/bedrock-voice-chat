use bvc_relay::node::NodeIdentity;
use bvc_relay::peer::{Handshake, PeerAuthority, PeerEndpoint, PeerError, PeerScope};
use common::structs::relay::Capability;
use common::structs::relay::wire::control::{Accept, RefuseReason};
use iroh::{EndpointAddr, PublicKey};
use tempfile::TempDir;

// Standing in for whatever owns the decision — a server's config table, or
// something else entirely.
//
// `node: None` authorizes anyone, which isolates the world-narrowing tests from
// the identity ones. An empty `filter` narrows nothing, which is the ordinary
// case: a block naming only a peer link.
struct TestAuthority {
    node: Option<PublicKey>,
    filter: Vec<String>,
}

impl PeerAuthority for TestAuthority {
    fn authorize(&self, node: &PublicKey, declared: &[String]) -> Option<PeerScope> {
        if self.node.is_some_and(|expected| expected != *node) {
            return None;
        }

        let worlds = if self.filter.is_empty() {
            declared.to_vec()
        } else {
            declared
                .iter()
                .filter(|world| self.filter.contains(world))
                .cloned()
                .collect()
        };

        Some(PeerScope {
            worlds,
            capabilities: vec![Capability::CarrySpeakers],
        })
    }
}

// Two endpoints on loopback, dialed by explicit address. No relay, no address
// lookup, no pkarr — a test that reaches the network is a bug.
async fn endpoint(dir: &TempDir) -> PeerEndpoint {
    let path = dir.path().to_str().expect("utf-8 path");
    let identity = NodeIdentity::load_or_create(path).expect("identity");
    PeerEndpoint::bind(&identity, None).await.expect("bind")
}

fn loopback_addr(endpoint: &PeerEndpoint) -> EndpointAddr {
    let mut addr = EndpointAddr::new(endpoint.node_id());
    for socket in endpoint.endpoint().bound_sockets() {
        if socket.is_ipv4() {
            addr = addr.with_ip_addr(std::net::SocketAddr::new(
                std::net::Ipv4Addr::LOCALHOST.into(),
                socket.port(),
            ));
        }
    }
    addr
}

// Runs one full handshake and returns what the dialer was told it holds.
async fn handshake_between(filter: Vec<String>, declared: Vec<String>) -> Result<Accept, PeerError> {
    let acceptor_dir = TempDir::new().expect("tempdir");
    let dialer_dir = TempDir::new().expect("tempdir");
    let acceptor = endpoint(&acceptor_dir).await;
    let dialer = endpoint(&dialer_dir).await;

    let authority = TestAuthority { node: None, filter };
    let addr = loopback_addr(&acceptor);
    let listening = acceptor.endpoint().clone();

    // The connection is returned rather than dropped at the end of the task:
    // dropping it closes the link, and the reply can still be in flight.
    let server = tokio::spawn(async move {
        let incoming = listening.accept().await.expect("incoming");
        let conn = incoming.await.expect("connection");
        let outcome = Handshake::accept(&conn, &authority).await;
        (outcome, conn)
    });

    let conn = dialer
        .endpoint()
        .connect(addr, PeerEndpoint::ALPN)
        .await
        .expect("dial");

    let dialed = Handshake::dial(&conn, declared).await;
    let _ = server.await.expect("join");
    dialed
}

#[tokio::test]
async fn an_authorized_peer_is_accepted_and_told_its_scope() {
    let acceptor_dir = TempDir::new().expect("tempdir");
    let dialer_dir = TempDir::new().expect("tempdir");
    let acceptor = endpoint(&acceptor_dir).await;
    let dialer = endpoint(&dialer_dir).await;

    let authority = TestAuthority {
        node: Some(dialer.node_id()),
        filter: Vec::new(),
    };
    let addr = loopback_addr(&acceptor);
    let listening = acceptor.endpoint().clone();

    let server = tokio::spawn(async move {
        let incoming = listening.accept().await.expect("incoming");
        let conn = incoming.await.expect("connection");
        let outcome = Handshake::accept(&conn, &authority).await;
        (outcome, conn)
    });

    let conn = dialer
        .endpoint()
        .connect(addr, PeerEndpoint::ALPN)
        .await
        .expect("dial");
    let accepted = Handshake::dial(&conn, vec!["W1".to_string(), "W2".to_string()])
        .await
        .expect("handshake succeeds");

    assert_eq!(accepted.worlds, vec!["W1".to_string(), "W2".to_string()]);
    assert_eq!(accepted.capabilities, vec![Capability::CarrySpeakers]);
    let (outcome, _held) = server.await.expect("join");
    outcome.expect("acceptor side succeeds");
}

#[tokio::test]
async fn an_unauthorized_peer_is_refused_with_the_reason() {
    let acceptor_dir = TempDir::new().expect("tempdir");
    let dialer_dir = TempDir::new().expect("tempdir");
    let stranger_dir = TempDir::new().expect("tempdir");
    let acceptor = endpoint(&acceptor_dir).await;
    let dialer = endpoint(&dialer_dir).await;

    // The authority names somebody else entirely.
    let other = NodeIdentity::load_or_create(stranger_dir.path().to_str().expect("path"))
        .expect("identity");
    let authority = TestAuthority {
        node: Some(other.node_id()),
        filter: Vec::new(),
    };
    let addr = loopback_addr(&acceptor);
    let listening = acceptor.endpoint().clone();

    let server = tokio::spawn(async move {
        let incoming = listening.accept().await.expect("incoming");
        let conn = incoming.await.expect("connection");
        let outcome = Handshake::accept(&conn, &authority).await;
        (outcome, conn)
    });

    let conn = dialer
        .endpoint()
        .connect(addr, PeerEndpoint::ALPN)
        .await
        .expect("dial");
    let outcome = Handshake::dial(&conn, vec!["W1".to_string()]).await;

    // The reason is asserted, not merely the failure: a bare `is_err` would pass
    // just as well on a dropped connection, which is how this test first passed
    // while the handshake was not working at all.
    match outcome {
        Err(PeerError::Refused(reason)) => assert_eq!(reason, RefuseReason::NotAuthorized),
        other => panic!("expected an explicit refusal, got {other:?}"),
    }

    let (outcome, _held) = server.await.expect("join");
    assert!(outcome.is_err());
}

// The declaration is the default answer: an operator who names only a node id
// gets whatever that peer says it hosts.
#[tokio::test]
async fn an_unfiltered_authority_returns_the_declared_worlds() {
    let accepted = handshake_between(
        Vec::new(),
        vec!["alpha".to_string(), "beta".to_string()],
    )
    .await
    .expect("accepted");

    assert_eq!(
        accepted.worlds,
        vec!["alpha".to_string(), "beta".to_string()]
    );
}

// A configured filter narrows, and the dialer is told what survived.
#[tokio::test]
async fn a_filter_narrows_the_declaration_to_the_intersection() {
    let accepted = handshake_between(
        vec!["alpha".to_string()],
        vec!["alpha".to_string(), "beta".to_string()],
    )
    .await
    .expect("accepted");

    assert_eq!(accepted.worlds, vec!["alpha".to_string()]);
}

// An empty intersection is the failure this design exists to remove: a link that
// connects, reports healthy, and drops every frame.
#[tokio::test]
async fn a_declaration_the_filter_excludes_entirely_is_refused() {
    let outcome = handshake_between(vec!["alpha".to_string()], vec!["beta".to_string()]).await;

    match outcome {
        Err(PeerError::Refused(reason)) => assert_eq!(reason, RefuseReason::NoSharedWorld),
        other => panic!("expected an explicit refusal, got {other:?}"),
    }
}

#[tokio::test]
async fn a_dialer_declaring_no_worlds_is_refused() {
    let outcome = handshake_between(Vec::new(), Vec::new()).await;

    match outcome {
        Err(PeerError::Refused(reason)) => assert_eq!(reason, RefuseReason::NoSharedWorld),
        other => panic!("expected an explicit refusal, got {other:?}"),
    }
}
