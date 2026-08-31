use bvc_relay::node::NodeIdentity;
use bvc_relay::peer::{Handshake, PeerAuthority, PeerEndpoint, PeerError, PeerScope, RedeemResult};
use common::structs::relay::Capability;
use common::structs::relay::wire::control::{Enrolled, RefuseReason};
use iroh::{EndpointAddr, PublicKey};
use tempfile::TempDir;

// An authority that grants exactly one code and refuses everything else, so the test
// exercises the handshake's branching rather than a real pairing store.
//
// `authorize` always refuses: an enrolling node is by definition one that holds no grant,
// and answering it from `authorize` would hide which path the handshake took.
struct OneCode {
    code: String,
}

#[async_trait::async_trait]
impl PeerAuthority for OneCode {
    fn authorize(&self, _node: &PublicKey, _declared: &[String]) -> Option<PeerScope> {
        None
    }

    async fn redeem(&self, _node: &PublicKey, code: &str, declared: &[String]) -> RedeemResult {
        if code == self.code {
            RedeemResult::Granted(PeerScope {
                worlds: declared.to_vec(),
                capabilities: vec![Capability::CarrySpeakers],
            })
        } else {
            RedeemResult::Refused(RefuseReason::UnknownCode)
        }
    }
}

// Two endpoints on loopback, dialed by explicit address. No relay, no address lookup, no
// pkarr — a test that reaches the network is a bug.
async fn endpoint(dir: &TempDir) -> PeerEndpoint {
    let path = dir.path().to_str().expect("utf-8 path");
    let identity = NodeIdentity::load_or_create(path).expect("identity");
    PeerEndpoint::bind(&identity).await.expect("bind")
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

// Runs one full enrolment and returns what the dialer was told.
async fn enrol_with(minted: &str, typed: &str) -> Result<Enrolled, PeerError> {
    let acceptor_dir = TempDir::new().expect("tempdir");
    let dialer_dir = TempDir::new().expect("tempdir");
    let acceptor = endpoint(&acceptor_dir).await;
    let dialer = endpoint(&dialer_dir).await;

    let authority = OneCode {
        code: minted.to_string(),
    };
    let addr = loopback_addr(&acceptor);
    let listening = acceptor.endpoint().clone();

    // The connection is returned rather than dropped at the end of the task: dropping it
    // closes the link, and the reply can still be in flight.
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

    let dialed = Handshake::enrol(&conn, vec!["W1".to_string()], typed.to_string()).await;
    let _ = server.await.expect("join");
    dialed
}

#[tokio::test]
async fn a_valid_code_is_accepted_from_a_node_with_no_grant() {
    let enrolled = enrol_with("K7M49QTR", "K7M49QTR")
        .await
        .expect("enrolment accepted");

    assert_eq!(enrolled.worlds, vec!["W1".to_string()]);
    assert_eq!(enrolled.capabilities, vec![Capability::CarrySpeakers]);
}

// A refusal is sent before the error is returned, so the dialer learns why rather than
// seeing a bare close it would read as a network fault and retry.
#[tokio::test]
async fn an_unknown_code_is_refused_with_its_reason() {
    let outcome = enrol_with("K7M49QTR", "ZZZZZZZZ").await;

    assert!(matches!(
        outcome,
        Err(PeerError::Refused(RefuseReason::UnknownCode))
    ));
}

// The default must refuse, so a third-party PeerAuthority that has not been updated
// cannot admit an enrolling peer by omission.
#[tokio::test]
async fn an_authority_that_does_not_override_redeem_refuses() {
    struct NeverGrants;

    #[async_trait::async_trait]
    impl PeerAuthority for NeverGrants {
        fn authorize(&self, _node: &PublicKey, _declared: &[String]) -> Option<PeerScope> {
            None
        }
    }

    let dir = TempDir::new().expect("tempdir");
    let node = endpoint(&dir).await.node_id();

    let outcome = NeverGrants
        .redeem(&node, "K7M49QTR", &["W1".to_string()])
        .await;

    assert!(matches!(
        outcome,
        RedeemResult::Refused(RefuseReason::UnknownCode)
    ));
}
