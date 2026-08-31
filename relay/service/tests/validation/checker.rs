use std::sync::Arc;

use bvc_relay_service::config::DiscordConfig;
use bvc_relay_service::db::Db;
use bvc_relay_service::discord::{FixedMemberSource, MemberSource};
use bvc_relay_service::dns::{CloudflareApi, RecordingApi, ZoneWriter};
use bvc_relay_service::registry::RegistryService;
use bvc_relay_service::validation::{ValidationChecker, ValidationOutcome};

struct Fixture {
    checker: Arc<ValidationChecker>,
    registry: Arc<RegistryService>,
    zone: Arc<ZoneWriter>,
    recording: Arc<RecordingApi>,
    node: String,
    name: String,
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

    let checker = ValidationChecker::new_shared(conn, registry.clone(), zone.clone());
    Fixture {
        checker,
        registry,
        zone,
        recording,
        node: "node-a".to_string(),
        name,
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

    let outcome = f.checker.evaluate(&f.node, true, None).await.expect("passes");

    assert_eq!(outcome, ValidationOutcome::Passed);

    let next = f
        .checker
        .evaluate(&f.node, false, None)
        .await
        .expect("fails again");
    assert_eq!(next, ValidationOutcome::Failed { consecutive: 1 });
}

// Failures suspend only after the threshold. A single missed check is an outage, not
// an abandonment.
#[tokio::test]
async fn a_registration_is_suspended_only_after_the_threshold() {
    let f = fixture().await;

    for expected in 1..ValidationChecker::FAILURE_THRESHOLD {
        let outcome = f.checker.evaluate(&f.node, false, None).await.expect("fails");
        assert_eq!(
            outcome,
            ValidationOutcome::Failed {
                consecutive: expected
            }
        );
        assert!(f.registry.name_for(&f.node).await.expect("lookup").is_some());
    }

    let outcome = f.checker.evaluate(&f.node, false, None).await.expect("fails");

    assert_eq!(outcome, ValidationOutcome::Suspended);
    assert_eq!(f.registry.name_for(&f.node).await.expect("lookup"), None);
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

// Suspension withdraws what the relay published. A name that keeps resolving after
// its registration was suspended still points at a host the relay no longer vouches
// for.
#[tokio::test]
async fn suspension_withdraws_the_published_records() {
    let f = fixture().await;
    f.zone
        .publish_a(&f.name, "203.0.113.10")
        .await
        .expect("publishes an address");
    assert_eq!(f.recording.live_ids().len(), 1);

    for _ in 0..ValidationChecker::FAILURE_THRESHOLD {
        f.checker.evaluate(&f.node, false, None).await.expect("fails");
    }

    assert!(
        f.recording.live_ids().is_empty(),
        "a suspended name must stop resolving"
    );
}

// An unknown node is an error rather than a silent pass. A validation loop that
// quietly skipped rows it could not find would report every registration healthy
// after a database that lost them.
#[tokio::test]
async fn evaluating_an_unknown_node_is_an_error() {
    let f = fixture().await;

    assert!(f.checker.evaluate("node-unknown", true, None).await.is_err());
}
