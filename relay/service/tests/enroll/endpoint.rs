use std::sync::Arc;
use std::time::Duration;

use bvc_relay::node::{NodeIdentity, PeerTicket};
use bvc_relay::peer::PeerEndpoint;
use bvc_relay_service::budget::WeeklyBudget;
use bvc_relay_service::config::DiscordConfig;
use bvc_relay_service::db::Db;
use bvc_relay_service::discord::{FixedMemberSource, MemberSource};
use bvc_relay_service::dns::{CloudflareApi, RecordingApi, ZoneWriter};
use bvc_relay_service::registry::RegistryEndpoint;
use bvc_relay_service::registry::RegistryService;
use common::structs::relay::enroll::{EnrollFrame, EnrollRefuseReason, EnrollVersion};
use common::structs::relay::wire::Framing;
use iroh::endpoint::Connection;
use tempfile::TempDir;

fn discord_config() -> DiscordConfig {
    DiscordConfig {
        guild_id: "guild".to_string(),
        bot_token: "bot".to_string(),
        client_id: "client".to_string(),
        client_secret: "secret".to_string(),
        qualifying_role_ids: vec!["role-a".to_string()],
    }
}

pub struct Harness {
    _dir: TempDir,
    _client_dir: TempDir,
    // Held for the life of the harness. Dropping the endpoint closes the socket the
    // connection rides, and every request would then fail as a transport error.
    _client: PeerEndpoint,
    registry: Arc<RegistryService>,
    recording: Arc<RecordingApi>,
    conn: Connection,
    // The registry's own peer link. The observe tests need a live registry and none
    // of the enrollment state, so they dial this rather than build a second one.
    pub ticket: String,
    // Held so the accept loop stays alive for the life of the harness, and so a test
    // can read the session table.
    endpoint: std::sync::Arc<RegistryEndpoint>,
}

// A live registry, for tests that need one but none of the enrollment state.
pub async fn registry_harness(roles: Vec<String>) -> Harness {
    Harness::start(roles).await
}

impl Harness {
    // A relay endpoint and a client dialled to it over loopback. No relay URL on
    // either side: the ticket carries a loopback address, which is what makes an
    // in-process test possible at all.
    pub async fn start(roles: Vec<String>) -> Self {
        Self::start_with_ceiling(roles, 50).await
    }

    pub async fn start_with_ceiling(roles: Vec<String>, ceiling: u32) -> Self {
        let dir = TempDir::new().expect("tempdir");
        let client_dir = TempDir::new().expect("tempdir");

        let db = Arc::new(Db::connect("sqlite::memory:").await.expect("connects"));
        let registry = RegistryService::new_shared(
            db.clone(),
            discord_config(),
            MemberSource::Fixed(FixedMemberSource::new(roles)),
        );
        let recording = Arc::new(RecordingApi::new());
        let zone = Arc::new(ZoneWriter::new(
            db.clone(),
            CloudflareApi::Recording(recording.clone()),
            "bedrockvc.stream".to_string(),
        ));

        let identity =
            NodeIdentity::load_or_create(dir.path().to_str().expect("path")).expect("identity");
        let budget = WeeklyBudget::new_shared(db, ceiling);
        let endpoint = RegistryEndpoint::bind(&identity, registry.clone(), zone, budget, None)
            .await
            .expect("binds");
        endpoint.spawn_accept_loop();

        let ticket = endpoint.ticket().await.expect("ticket");
        let addr = PeerTicket::parse(&ticket).expect("parses");

        let client_identity = NodeIdentity::load_or_create(
            client_dir.path().to_str().expect("path"),
        )
        .expect("identity");
        let client = PeerEndpoint::bind_with_alpns(
            &client_identity,
            None,
            vec![RegistryEndpoint::ALPN.to_vec()],
        )
        .await
        .expect("binds a client");

        let conn = tokio::time::timeout(
            Duration::from_secs(10),
            client.endpoint().connect(addr, RegistryEndpoint::ALPN),
        )
        .await
        .expect("the dial completes within the timeout")
        .expect("dials");

        Self {
            _dir: dir,
            _client_dir: client_dir,
            _client: client,
            registry,
            recording,
            conn,
            ticket,
            endpoint,
        }
    }

    // The registry's live session table, so a test can assert what a protocol did or
    // did not leave behind.
    pub fn sessions(&self) -> &std::sync::Arc<bvc_relay_service::enroll::EnrollSessions> {
        self.endpoint.sessions()
    }

    // The client's own node id, as the relay records it. Read from the connection
    // rather than tracked separately, so the test asserts against the identity the
    // relay actually authenticated.
    fn node_id(&self) -> String {
        self._client.node_id().to_string()
    }

    async fn request(&self, frame: &EnrollFrame) -> EnrollFrame {
        let (mut send, mut recv) = self.conn.open_bi().await.expect("opens a stream");
        send.write_all(&Framing::encode(frame).expect("encodes"))
            .await
            .expect("writes");
        send.finish().expect("finishes");

        let mut header = [0u8; Framing::HEADER_LEN];
        recv.read_exact(&mut header).await.expect("reads a header");
        let len = Framing::payload_len(&header).expect("a valid length");
        let mut payload = vec![0u8; len];
        recv.read_exact(&mut payload).await.expect("reads a body");
        Framing::decode(&payload).expect("decodes")
    }
}

// The ALPN is a cross-version contract with every deployed server. A change stops
// every existing server enrolling, and the symptom is a dial that times out rather
// than one reporting a mismatch.
#[test]
fn the_enrollment_alpn_is_pinned() {
    assert_eq!(RegistryEndpoint::ALPN, b"bvc-enroll/1");
}

#[tokio::test]
async fn a_version_both_sides_speak_is_agreed() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;

    let reply = harness
        .request(&EnrollFrame::Hello {
            versions: EnrollVersion::SUPPORTED.to_vec(),
        })
        .await;

    assert!(matches!(reply, EnrollFrame::Ready { .. }));
}

#[tokio::test]
async fn a_version_neither_side_speaks_is_refused() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;

    let reply = harness
        .request(&EnrollFrame::Hello {
            versions: vec![EnrollVersion(9999)],
        })
        .await;

    assert_eq!(
        reply,
        EnrollFrame::Refuse {
            reason: EnrollRefuseReason::NoCommonVersion
        }
    );
}

// The whole enrollment path, end to end over a real connection: a token minted for
// an entitled member is redeemed by whichever node presents it, and the assigned
// name comes back.
#[tokio::test]
async fn an_entitled_token_is_redeemed_for_a_name() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;
    let token = harness
        .registry
        .issue_token("member-1")
        .await
        .expect("issues");

    let reply = harness.request(&EnrollFrame::Enroll { token }).await;

    match reply {
        EnrollFrame::Assigned { name } => assert!(!name.is_empty()),
        other => panic!("expected an assignment, got {other:?}"),
    }
}

#[tokio::test]
async fn an_unknown_token_is_refused_with_the_reason() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;

    let reply = harness
        .request(&EnrollFrame::Enroll {
            token: "bvcenroll-nonsense".to_string(),
        })
        .await;

    assert_eq!(
        reply,
        EnrollFrame::Refuse {
            reason: EnrollRefuseReason::UnknownToken
        }
    );
}

// The security invariant this endpoint exists to hold: a node may publish a
// challenge for its own name and no other. Without it, any enrolled operator could
// complete a certificate order for anyone else's name.
#[tokio::test]
async fn a_node_cannot_publish_a_challenge_for_another_name() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;
    let token = harness
        .registry
        .issue_token("member-1")
        .await
        .expect("issues");
    harness.request(&EnrollFrame::Enroll { token }).await;

    let reply = harness
        .request(&EnrollFrame::PublishTxt {
            name: "somebody-elses-name".to_string(),
            value: "challenge".to_string(),
        })
        .await;

    assert_eq!(
        reply,
        EnrollFrame::Refuse {
            reason: EnrollRefuseReason::NameNotOwned
        }
    );
    assert!(
        harness.recording.live_ids().is_empty(),
        "a refused publish must write nothing"
    );
}

// A node with no registration at all cannot publish either. This is the case an
// unenrolled or suspended server presents.
#[tokio::test]
async fn an_unregistered_node_cannot_publish_a_challenge() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;

    let reply = harness
        .request(&EnrollFrame::PublishTxt {
            name: "any-name".to_string(),
            value: "challenge".to_string(),
        })
        .await;

    assert_eq!(
        reply,
        EnrollFrame::Refuse {
            reason: EnrollRefuseReason::NameNotOwned
        }
    );
}

// The assigned name has to be fully qualified.
//
// A bare label is not a hostname: a server publishes what it is given as its own name,
// presents it as a SAN, and asks the certificate authority to sign it — which fails at
// the order, because a label is not a domain. The failure surfaces as "creating ACME
// order" and names nothing about the name itself.
#[tokio::test]
async fn an_assigned_name_is_fully_qualified() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;
    let token = harness
        .registry
        .issue_token("member-1")
        .await
        .expect("issues");

    let name = match harness.request(&EnrollFrame::Enroll { token }).await {
        EnrollFrame::Assigned { name } => name,
        other => panic!("expected an assignment, got {other:?}"),
    };

    assert!(
        name.ends_with(".bedrockvc.stream"),
        "the wire name must carry the zone, got {name}"
    );
    assert_eq!(
        name.matches("bedrockvc.stream").count(),
        1,
        "the zone must be appended once, got {name}"
    );
}

// A node publishing for the name it actually holds reaches the zone.
#[tokio::test]
async fn a_node_publishes_a_challenge_for_its_own_name() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;
    let token = harness
        .registry
        .issue_token("member-1")
        .await
        .expect("issues");
    let name = match harness.request(&EnrollFrame::Enroll { token }).await {
        EnrollFrame::Assigned { name } => name,
        other => panic!("expected an assignment, got {other:?}"),
    };

    let reply = harness
        .request(&EnrollFrame::PublishTxt {
            name: name.clone(),
            value: "challenge-value".to_string(),
        })
        .await;

    assert_eq!(reply, EnrollFrame::TxtPublished);
    assert_eq!(
        harness.recording.created_names(),
        vec![format!("_acme-challenge.{name}")]
    );
}

// A first issuance is refused once the weekly ceiling is spent, rather than
// attempted. The authority would reject it, burning the order and delaying the
// operator further — and the relay is the only component that can see the ceiling.
#[tokio::test]
async fn a_first_issuance_is_refused_once_the_weekly_ceiling_is_spent() {
    let harness = Harness::start_with_ceiling(vec!["role-a".to_string()], 0).await;
    let token = harness
        .registry
        .issue_token("member-1")
        .await
        .expect("issues");
    let name = match harness.request(&EnrollFrame::Enroll { token }).await {
        EnrollFrame::Assigned { name } => name,
        other => panic!("expected an assignment, got {other:?}"),
    };

    let reply = harness
        .request(&EnrollFrame::PublishTxt {
            name,
            value: "challenge".to_string(),
        })
        .await;

    assert_eq!(
        reply,
        EnrollFrame::Refuse {
            reason: EnrollRefuseReason::Internal
        }
    );
    assert!(
        harness.recording.live_ids().is_empty(),
        "a refused issuance must write nothing to the zone"
    );
}

// The defect this guards: an address declared over the wire must be recorded against
// the registration as well as published. The daily pass reads the stored column to
// decide whether to bind the record to the node, so an address published without
// being recorded is one nothing ever verifies — and the relay would front whatever
// host it points at, from its own zone, forever.
#[tokio::test]
async fn a_declared_address_is_both_recorded_and_published() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;
    let token = harness
        .registry
        .issue_token("member-1")
        .await
        .expect("issues");
    let name = match harness.request(&EnrollFrame::Enroll { token }).await {
        EnrollFrame::Assigned { name } => name,
        other => panic!("expected an assignment, got {other:?}"),
    };

    let reply = harness
        .request(&EnrollFrame::DeclareAddress {
            address: "8.8.8.8".to_string(),
        })
        .await;

    assert_eq!(reply, EnrollFrame::TxtPublished);
    assert_eq!(
        harness
            .registry
            .declared_address(&harness.node_id())
            .await
            .expect("lookup"),
        Some("8.8.8.8".to_string()),
        "the address must be recorded, not only published"
    );
    assert_eq!(
        harness.recording.created_names(),
        vec![name.clone()]
    );
}

// A node that has not enrolled cannot put an address record into the relay's zone.
#[tokio::test]
async fn an_unregistered_node_cannot_declare_an_address() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;

    let reply = harness
        .request(&EnrollFrame::DeclareAddress {
            address: "8.8.8.8".to_string(),
        })
        .await;

    assert_eq!(
        reply,
        EnrollFrame::Refuse {
            reason: EnrollRefuseReason::NotRegistered
        }
    );
    assert!(harness.recording.live_ids().is_empty());
}
