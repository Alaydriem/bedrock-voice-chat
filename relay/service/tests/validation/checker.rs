use std::sync::Arc;

use bvc_relay_service::config::DiscordConfig;
use bvc_relay_service::db::Db;
use bvc_relay_service::discord::{FixedMemberSource, MemberSource};
use bvc_relay_service::dns::{CloudflareApi, RecordingApi, ZoneWriter};
use bvc_relay_service::entity::registration;
use bvc_relay_service::registry::RegistryService;
use sea_orm::{ConnectionTrait, DatabaseConnection, EntityTrait};
use bvc_relay_service::validation::{ValidationChecker, ValidationOutcome};

struct Fixture {
    checker: Arc<ValidationChecker>,
    registry: Arc<RegistryService>,
    zone: Arc<ZoneWriter>,
    recording: Arc<RecordingApi>,
    node: String,
    name: String,
    conn: Arc<DatabaseConnection>,
}

impl Fixture {
    async fn row(&self) -> registration::Model {
        registration::Entity::find_by_id(self.node.as_str())
            .one(self.conn.as_ref())
            .await
            .expect("queries")
            .expect("row exists")
    }
}

async fn fixture() -> Fixture {
    let conn = Arc::new(Db::connect("sqlite::memory:").await.expect("connects"));
    let discord = DiscordConfig {
        guild_id: "guild".to_string(),
        bot_token: "bot".to_string(),
        client_id: "client".to_string(),
        client_secret: "secret".to_string(),
        qualifying_role_ids: vec!["role-a".to_string()],
    };
    let registry = RegistryService::new_shared(
        conn.clone(),
        discord,
        MemberSource::Fixed(FixedMemberSource::new(vec!["role-a".to_string()])),
    );
    let recording = Arc::new(RecordingApi::new());
    let zone = Arc::new(ZoneWriter::new(
        conn.clone(),
        CloudflareApi::Recording(recording.clone()),
        "bedrockvc.stream".to_string(),
    ));

    let token = registry.issue_token("member-1").await.expect("issues");
    let name = registry.redeem(&token, "node-a").await.expect("redeems");

    let checker = ValidationChecker::new_shared(conn.clone(), registry.clone(), zone.clone());
    Fixture {
        checker,
        registry,
        zone,
        recording,
        node: "node-a".to_string(),
        name,
        conn,
    }
}

// A node that answers its challenge passes, and a passing check clears any earlier
// failures rather than leaving them to accumulate across unrelated outages.
#[tokio::test]
async fn a_node_that_answers_passes_and_clears_earlier_failures() {
    let f = fixture().await;
    f.checker
        .evaluate(&f.node, false, None)
        .await
        .expect("first failure");

    let outcome = f
        .checker
        .evaluate(&f.node, true, None)
        .await
        .expect("passes");

    assert_eq!(outcome, ValidationOutcome::Passed);

    let next = f
        .checker
        .evaluate(&f.node, false, None)
        .await
        .expect("fails again");
    assert_eq!(next, ValidationOutcome::Failed { consecutive: 1 });
}

#[tokio::test]
async fn a_registration_is_revoked_only_after_the_threshold() {
    let f = fixture().await;

    for expected in 1..ValidationChecker::FAILURE_THRESHOLD {
        let outcome = f
            .checker
            .evaluate(&f.node, false, None)
            .await
            .expect("fails");
        assert_eq!(
            outcome,
            ValidationOutcome::Failed {
                consecutive: expected
            }
        );
        assert!(
            f.registry
                .name_for(&f.node)
                .await
                .expect("lookup")
                .is_some()
        );
    }

    let outcome = f
        .checker
        .evaluate(&f.node, false, None)
        .await
        .expect("fails");

    assert_eq!(outcome, ValidationOutcome::Revoked);
}

// A registration that published an address must bind that address to the node. A
// node whose own challenge answers but whose declared address serves something else
// is fronting a host it does not control from the relay's zone.
#[tokio::test]
async fn a_declared_address_that_does_not_serve_the_nonce_counts_as_a_failure() {
    let f = fixture().await;

    let outcome = f
        .checker
        .evaluate(&f.node, true, Some(false))
        .await
        .expect("evaluates");

    assert_eq!(outcome, ValidationOutcome::Failed { consecutive: 1 });
}

// A registration with no published address skips the address half entirely. Nothing
// is published for it, so there is nothing to bind.
#[tokio::test]
async fn a_registration_with_no_address_passes_on_identity_alone() {
    let f = fixture().await;

    let outcome = f
        .checker
        .evaluate(&f.node, true, None)
        .await
        .expect("evaluates");

    assert_eq!(outcome, ValidationOutcome::Passed);
}

#[tokio::test]
async fn revocation_withdraws_only_the_address_record() {
    let f = fixture().await;
    f.zone
        .publish_txt(&f.name, "challenge")
        .await
        .expect("publishes a challenge");
    f.zone
        .publish_address(&f.name, "203.0.113.10".parse().expect("an address"))
        .await
        .expect("publishes an address");

    let mut outcome = ValidationOutcome::Passed;
    for _ in 0..ValidationChecker::FAILURE_THRESHOLD {
        outcome = f
            .checker
            .evaluate(&f.node, false, None)
            .await
            .expect("fails");
    }

    assert_eq!(outcome, ValidationOutcome::Revoked);
    assert_eq!(f.zone.open_challenges(&f.name).await.expect("counts"), 1);
    assert_eq!(f.recording.live_ids().len(), 1);
}

#[tokio::test]
async fn a_revoked_registration_that_passes_is_recovered_and_published() {
    let f = fixture().await;
    f.registry
        .record_check_in(&f.node, Some("203.0.113.5".parse().unwrap()), 8443)
        .await
        .expect("checks in");
    f.registry.revoke(&f.node).await.expect("revokes");

    let outcome = f
        .checker
        .evaluate(&f.node, true, Some(true))
        .await
        .expect("evaluates");

    assert_eq!(outcome, ValidationOutcome::Recovered);
    assert_eq!(
        f.registry.name_for(&f.node).await.expect("lookup"),
        Some(f.name.clone())
    );
    assert_eq!(
        f.recording.created_records().last(),
        Some(&(
            "A".to_string(),
            format!("{}.bedrockvc.stream", f.name),
            "203.0.113.5".to_string()
        ))
    );
}

#[tokio::test]
async fn a_suspended_registration_is_left_untouched() {
    let f = fixture().await;
    f.zone
        .publish_address(&f.name, "203.0.113.10".parse().expect("an address"))
        .await
        .expect("publishes an address");
    f.conn
        .execute_unprepared("UPDATE registration SET state = 'suspended'")
        .await
        .expect("suspends");
    let before = f.row().await;

    for _ in 0..ValidationChecker::FAILURE_THRESHOLD {
        f.checker
            .evaluate(&f.node, false, None)
            .await
            .expect("evaluates");
    }

    assert_eq!(f.row().await, before);
    assert!(f.zone.has_records(&f.name).await.expect("checks"));
}

#[tokio::test]
async fn a_revoked_registration_that_fails_stays_revoked_and_unpublished() {
    let f = fixture().await;
    f.registry
        .record_check_in(&f.node, Some("203.0.113.5".parse().unwrap()), 8443)
        .await
        .expect("checks in");
    f.registry.revoke(&f.node).await.expect("revokes");

    let outcome = f
        .checker
        .evaluate(&f.node, false, None)
        .await
        .expect("evaluates");

    assert_eq!(outcome, ValidationOutcome::Failed { consecutive: 1 });
    assert!(f.recording.created_records().is_empty());
}

// An unknown node is an error rather than a silent pass. A validation loop that
// quietly skipped rows it could not find would report every registration healthy
// after a database that lost them.
#[tokio::test]
async fn evaluating_an_unknown_node_is_an_error() {
    let f = fixture().await;

    assert!(
        f.checker
            .evaluate("node-unknown", true, None)
            .await
            .is_err()
    );
}
