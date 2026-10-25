use std::sync::Arc;
use std::time::Duration;

use bvc_relay::node::{NodeIdentity, PeerTicket};
use bvc_relay::peer::PeerEndpoint;
use bvc_relay_service::budget::WeeklyBudget;
use bvc_relay_service::config::DiscordConfig;
use bvc_relay_service::db::Db;
use bvc_relay_service::discord::{FixedMemberSource, MemberSource};
use bvc_relay_service::dns::{CloudflareApi, RecordingApi, ZoneWriter};
use bvc_relay_service::entity::registration;
use bvc_relay_service::registry::{ClaimService, RegistryEndpoint, RegistryService};
use bvc_relay_service::scheduler::DailyScheduler;
use bvc_relay_service::validation::{AddressProbe, AddressVerifier, ValidationChecker};
use common::structs::relay::enroll::{EnrollFrame, EnrollRefuseReason, EnrollVersion};
use common::structs::relay::wire::Framing;
use iroh::endpoint::Connection;
use sea_orm::{ConnectionTrait, DatabaseConnection, EntityTrait};
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

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

pub struct Harness {
    _dir: TempDir,
    _client_dir: TempDir,
    _client: PeerEndpoint,
    registry: Arc<RegistryService>,
    recording: Arc<RecordingApi>,
    db: Arc<DatabaseConnection>,
    conn: Connection,
    pub ticket: String,
    _endpoint: Arc<RegistryEndpoint>,
}

// A live registry, for tests that need one but none of the enrollment state.
pub async fn registry_harness(roles: Vec<String>) -> Harness {
    Harness::start(roles).await
}

impl Harness {
    pub async fn start(roles: Vec<String>) -> Self {
        Self::start_with(roles.clone(), roles, 50).await
    }

    pub async fn start_with_ceiling(roles: Vec<String>, ceiling: u32) -> Self {
        Self::start_with(roles.clone(), roles, ceiling).await
    }

    // `scheduler_roles` are what a check-in's evaluation sees, as opposed to enrollment.
    pub async fn start_with(roles: Vec<String>, scheduler_roles: Vec<String>, ceiling: u32) -> Self {
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
        let scheduler = DailyScheduler::new_shared(
            db.clone(),
            ValidationChecker::new_shared(db.clone(), registry.clone(), zone.clone()),
            AddressVerifier::new_shared(AddressProbe::Fixed(false), zone.clone()),
            registry.clone(),
            zone.clone(),
            ClaimService::new_shared(db.clone()),
            MemberSource::Fixed(FixedMemberSource::new(scheduler_roles)),
            discord_config(),
        );

        let identity =
            NodeIdentity::load_or_create(dir.path().to_str().expect("path")).expect("identity");
        let budget = WeeklyBudget::new_shared(db.clone(), ceiling);
        let endpoint =
            RegistryEndpoint::bind(&identity, registry.clone(), zone, budget, scheduler, None)
                .await
                .expect("binds");
        endpoint.spawn_accept_loop();

        let ticket = endpoint.ticket().await.expect("ticket");
        let (client, conn) = Self::dial(&ticket, &client_dir).await;

        Self {
            _dir: dir,
            _client_dir: client_dir,
            _client: client,
            registry,
            recording,
            db,
            conn,
            ticket,
            _endpoint: endpoint,
        }
    }

    async fn dial(ticket: &str, dir: &TempDir) -> (PeerEndpoint, Connection) {
        let addr = PeerTicket::parse(ticket).expect("parses");
        let identity =
            NodeIdentity::load_or_create(dir.path().to_str().expect("path")).expect("identity");
        let client =
            PeerEndpoint::bind_with_alpns(&identity, None, vec![RegistryEndpoint::ALPN.to_vec()])
                .await
                .expect("binds a client");

        let conn = tokio::time::timeout(
            Duration::from_secs(10),
            client.endpoint().connect(addr, RegistryEndpoint::ALPN),
        )
        .await
        .expect("the dial completes within the timeout")
        .expect("dials");

        (client, conn)
    }

    async fn redial(&self) -> Connection {
        let addr = PeerTicket::parse(&self.ticket).expect("parses");
        tokio::time::timeout(
            Duration::from_secs(10),
            self._client.endpoint().connect(addr, RegistryEndpoint::ALPN),
        )
        .await
        .expect("the dial completes within the timeout")
        .expect("dials")
    }

    fn node_id(&self) -> String {
        self._client.node_id().to_string()
    }

    async fn request(&self, frame: &EnrollFrame) -> EnrollFrame {
        Self::request_on(&self.conn, frame).await
    }

    async fn request_on(conn: &Connection, frame: &EnrollFrame) -> EnrollFrame {
        let (mut send, mut recv) = conn.open_bi().await.expect("opens a stream");
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

    async fn enroll(&self) -> String {
        let token = self
            .registry
            .issue_token("member-1")
            .await
            .expect("issues");
        match self.request(&EnrollFrame::Enroll { token }).await {
            EnrollFrame::Assigned { name } => name,
            other => panic!("expected an assignment, got {other:?}"),
        }
    }

    async fn check_in(&self, address: Option<&str>, port: u16) -> EnrollFrame {
        self.request(&EnrollFrame::CheckIn {
            address: address.map(str::to_string),
            port,
        })
        .await
    }

    // The relay serves one stream at a time, so a reply means earlier requests finished.
    async fn barrier(&self) {
        self.request(&EnrollFrame::Hello {
            versions: EnrollVersion::SUPPORTED.to_vec(),
        })
        .await;
    }

    async fn row(&self) -> registration::Model {
        registration::Entity::find_by_id(self.node_id())
            .one(self.db.as_ref())
            .await
            .expect("reads")
            .expect("registered")
    }

    async fn sql(&self, statement: &str) {
        self.db
            .execute_unprepared(statement)
            .await
            .expect("updates");
    }
}

// Changing the ALPN stops every deployed server enrolling, as a dial timeout.
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
async fn a_version_one_hello_is_refused_with_no_common_version() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;

    let reply = harness
        .request(&EnrollFrame::Hello {
            versions: vec![EnrollVersion(1)],
        })
        .await;

    assert_eq!(
        reply,
        EnrollFrame::Refuse {
            reason: EnrollRefuseReason::NoCommonVersion
        }
    );
}

#[tokio::test]
async fn an_entitled_token_is_redeemed_for_a_name() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;

    assert!(!harness.enroll().await.is_empty());
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

#[tokio::test]
async fn a_node_cannot_publish_a_challenge_for_another_name() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;
    harness.enroll().await;

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
    assert!(harness.recording.live_ids().is_empty());
}

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

#[tokio::test]
async fn a_revoked_unentitled_node_cannot_publish_a_challenge() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;
    let name = harness.enroll().await;
    let node = harness.node_id();
    harness.registry.revoke(&node).await.expect("revokes");
    harness
        .registry
        .record_entitlement_check(&node, Some(false))
        .await
        .expect("records");

    let reply = harness
        .request(&EnrollFrame::PublishTxt {
            name,
            value: "challenge".to_string(),
        })
        .await;

    assert_eq!(
        reply,
        EnrollFrame::Refuse {
            reason: EnrollRefuseReason::NameNotOwned
        }
    );
    assert!(harness.recording.live_ids().is_empty());
}

#[tokio::test]
async fn an_assigned_name_is_fully_qualified() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;

    let name = harness.enroll().await;

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

#[tokio::test]
async fn a_node_publishes_a_challenge_for_its_own_name() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;
    let name = harness.enroll().await;

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

#[tokio::test]
async fn a_first_issuance_is_refused_once_the_weekly_ceiling_is_spent() {
    let harness = Harness::start_with_ceiling(vec!["role-a".to_string()], 0).await;
    let name = harness.enroll().await;

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
    assert!(harness.recording.live_ids().is_empty());
}

#[tokio::test]
async fn open_challenges_for_a_name_are_capped() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;
    let name = harness.enroll().await;

    for _ in 0..RegistryEndpoint::MAX_OPEN_CHALLENGES {
        let reply = harness
            .request(&EnrollFrame::PublishTxt {
                name: name.clone(),
                value: "value".to_string(),
            })
            .await;
        assert_eq!(reply, EnrollFrame::TxtPublished);
    }

    let reply = harness
        .request(&EnrollFrame::PublishTxt {
            name,
            value: "value".to_string(),
        })
        .await;

    assert_eq!(
        reply,
        EnrollFrame::Refuse {
            reason: EnrollRefuseReason::Internal
        }
    );
    assert_eq!(
        harness.recording.live_ids().len() as u64,
        RegistryEndpoint::MAX_OPEN_CHALLENGES
    );
}

#[tokio::test]
async fn a_check_in_publishes_the_address_and_returns_the_stored_nonce() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;
    let name = harness.enroll().await;

    let nonce = match harness.check_in(Some("8.8.8.8"), 8443).await {
        EnrollFrame::CheckedIn { nonce } => nonce,
        other => panic!("expected a check-in reply, got {other:?}"),
    };

    let row = harness.row().await;
    assert_eq!(row.checkin_nonce.map(String::into_bytes), Some(nonce));
    assert_eq!(row.declared_port, Some(8443));
    assert_eq!(
        harness.recording.created_records(),
        vec![("A".to_string(), name, "8.8.8.8".to_string())]
    );
}

#[tokio::test]
async fn an_ipv6_check_in_is_published_as_aaaa() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;
    let name = harness.enroll().await;

    let reply = harness.check_in(Some("2001:4860:4860::8888"), 443).await;

    assert!(matches!(reply, EnrollFrame::CheckedIn { .. }));
    assert_eq!(
        harness.recording.created_records(),
        vec![("AAAA".to_string(), name, "2001:4860:4860::8888".to_string())]
    );
}

#[tokio::test]
async fn a_check_in_that_is_not_an_ip_is_refused_and_changes_nothing() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;
    harness.enroll().await;
    let before = harness.row().await;

    let reply = harness.check_in(Some("attacker.example"), 443).await;

    assert_eq!(
        reply,
        EnrollFrame::Refuse {
            reason: EnrollRefuseReason::Internal
        }
    );
    assert_eq!(harness.row().await, before);
    assert!(harness.recording.live_ids().is_empty());
}

#[tokio::test]
async fn a_check_in_without_an_address_withdraws_the_address_record() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;
    harness.enroll().await;
    harness.check_in(Some("8.8.8.8"), 443).await;
    assert_eq!(harness.recording.live_ids().len(), 1);

    let reply = harness.check_in(None, 443).await;

    assert!(matches!(reply, EnrollFrame::CheckedIn { .. }));
    assert!(harness.recording.live_ids().is_empty());
    assert_eq!(harness.row().await.declared_address, None);
}

// Simulates a publish that failed after the check-in was recorded.
#[tokio::test]
async fn an_unchanged_check_in_republishes_a_missing_address_beside_an_open_challenge() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;
    let name = harness.enroll().await;
    harness.check_in(Some("8.8.8.8"), 443).await;
    harness
        .sql("DELETE FROM dns_record WHERE record_type = 'A'")
        .await;
    let reply = harness
        .request(&EnrollFrame::PublishTxt {
            name: name.clone(),
            value: "challenge".to_string(),
        })
        .await;
    assert_eq!(reply, EnrollFrame::TxtPublished);

    let reply = harness.check_in(Some("8.8.8.8"), 443).await;

    assert!(matches!(reply, EnrollFrame::CheckedIn { .. }));
    let addresses: Vec<_> = harness
        .recording
        .created_records()
        .into_iter()
        .filter(|(kind, _, _)| kind == "A")
        .collect();
    assert_eq!(
        addresses,
        vec![
            ("A".to_string(), name.clone(), "8.8.8.8".to_string()),
            ("A".to_string(), name, "8.8.8.8".to_string()),
        ]
    );
}

#[tokio::test]
async fn a_check_in_from_another_node_never_changes_this_registration() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;
    harness.enroll().await;
    let before = harness.row().await;

    let other_dir = TempDir::new().expect("tempdir");
    let (_other, other_conn) = Harness::dial(&harness.ticket, &other_dir).await;
    let reply = Harness::request_on(
        &other_conn,
        &EnrollFrame::CheckIn {
            address: Some("8.8.4.4".to_string()),
            port: 443,
        },
    )
    .await;

    assert_eq!(
        reply,
        EnrollFrame::Refuse {
            reason: EnrollRefuseReason::NotRegistered
        }
    );
    assert_eq!(harness.row().await, before);
    assert!(harness.recording.live_ids().is_empty());
}

#[tokio::test]
async fn a_revoked_node_check_in_is_recorded_and_not_published() {
    let harness =
        Harness::start_with(vec!["role-a".to_string()], vec!["role-z".to_string()], 50).await;
    harness.enroll().await;
    harness
        .registry
        .revoke(&harness.node_id())
        .await
        .expect("revokes");

    let nonce = match harness.check_in(Some("8.8.8.8"), 8443).await {
        EnrollFrame::CheckedIn { nonce } => nonce,
        other => panic!("expected a check-in reply, got {other:?}"),
    };
    harness.barrier().await;

    let row = harness.row().await;
    assert_eq!(row.state, "revoked");
    assert_eq!(row.checkin_nonce.map(String::into_bytes), Some(nonce));
    assert_eq!(row.declared_address.as_deref(), Some("8.8.8.8"));
    assert!(harness.recording.created_records().is_empty());
}

#[tokio::test]
async fn a_revoked_node_is_evaluated_at_most_once_per_cooldown() {
    let harness =
        Harness::start_with(vec!["role-a".to_string()], vec!["role-z".to_string()], 50).await;
    harness.enroll().await;
    harness
        .registry
        .revoke(&harness.node_id())
        .await
        .expect("revokes");
    let stale = now() - RegistryEndpoint::RECOVERY_COOLDOWN_SECONDS - 1;
    harness
        .sql(&format!(
            "UPDATE registration SET entitlement_checked_at = {stale}, validation_failures = 0"
        ))
        .await;

    harness.check_in(None, 443).await;
    harness.barrier().await;
    let first = harness.row().await;
    assert_ne!(first.entitlement_checked_at, Some(stale));
    assert_eq!(first.validation_failures, 1);

    harness.check_in(None, 443).await;
    harness.barrier().await;
    let second = harness.row().await;
    assert_eq!(second.entitlement_checked_at, first.entitlement_checked_at);
    assert_eq!(second.validation_failures, 1);
}

#[tokio::test]
async fn concurrent_check_ins_from_one_revoked_node_are_evaluated_once() {
    let harness =
        Harness::start_with(vec!["role-a".to_string()], vec!["role-z".to_string()], 50).await;
    harness.enroll().await;
    harness
        .registry
        .revoke(&harness.node_id())
        .await
        .expect("revokes");
    harness
        .sql("UPDATE registration SET entitlement_checked_at = NULL, validation_failures = 0")
        .await;
    let a = harness.redial().await;
    let b = harness.redial().await;
    let check_in = EnrollFrame::CheckIn {
        address: None,
        port: 443,
    };
    let hello = EnrollFrame::Hello {
        versions: EnrollVersion::SUPPORTED.to_vec(),
    };

    tokio::join!(
        Harness::request_on(&a, &check_in),
        Harness::request_on(&b, &check_in),
        Harness::request_on(&harness.conn, &check_in),
    );
    tokio::join!(
        Harness::request_on(&a, &hello),
        Harness::request_on(&b, &hello),
        Harness::request_on(&harness.conn, &hello),
    );

    assert_eq!(harness.row().await.validation_failures, 1);
}

#[tokio::test]
async fn a_state_the_relay_does_not_know_is_refused_on_check_in() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;
    harness.enroll().await;
    harness
        .sql("UPDATE registration SET state = 'blacklisted'")
        .await;

    let reply = harness.check_in(Some("8.8.8.8"), 443).await;

    assert_eq!(
        reply,
        EnrollFrame::Refuse {
            reason: EnrollRefuseReason::Suspended
        }
    );
    assert!(harness.recording.created_records().is_empty());
}

#[tokio::test]
async fn a_dns_fault_on_an_active_check_in_still_returns_the_stored_nonce() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;
    harness.enroll().await;
    harness.sql("DROP TABLE dns_record").await;

    let nonce = match harness.check_in(Some("8.8.8.8"), 443).await {
        EnrollFrame::CheckedIn { nonce } => nonce,
        other => panic!("expected a check-in reply, got {other:?}"),
    };

    assert_eq!(
        harness.row().await.checkin_nonce.map(String::into_bytes),
        Some(nonce)
    );
}

#[tokio::test]
async fn retired_frames_are_refused_as_internal() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;
    let frames = [
        EnrollFrame::Challenge { nonce: vec![1] },
        EnrollFrame::ChallengeReply { signature: vec![1] },
        EnrollFrame::DeclareAddress {
            address: "8.8.8.8".to_string(),
        },
    ];

    for frame in frames {
        assert_eq!(
            harness.request(&frame).await,
            EnrollFrame::Refuse {
                reason: EnrollRefuseReason::Internal
            }
        );
    }
}

#[tokio::test]
async fn a_suspended_node_check_in_is_refused() {
    let harness = Harness::start(vec!["role-a".to_string()]).await;
    harness.enroll().await;
    harness
        .sql("UPDATE registration SET state = 'suspended'")
        .await;

    let reply = harness.check_in(Some("8.8.8.8"), 443).await;

    assert_eq!(
        reply,
        EnrollFrame::Refuse {
            reason: EnrollRefuseReason::Suspended
        }
    );
    assert!(harness.recording.live_ids().is_empty());
}
