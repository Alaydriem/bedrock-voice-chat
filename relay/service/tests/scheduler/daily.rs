use std::net::IpAddr;
use std::sync::Arc;

use bvc_relay_service::config::DiscordConfig;
use bvc_relay_service::db::Db;
use bvc_relay_service::discord::{FixedMemberSource, MemberSource};
use bvc_relay_service::dns::{CloudflareApi, RecordingApi, ZoneWriter};
use bvc_relay_service::entity::registration;
use bvc_relay_service::registry::{ClaimService, RegistryService};
use bvc_relay_service::scheduler::DailyScheduler;
use bvc_relay_service::validation::{AddressProbe, AddressVerifier, ValidationChecker};
use sea_orm::{ConnectionTrait, DatabaseConnection, EntityTrait};

const ZONE: &str = "bedrockvc.stream";
const PUBLIC: &str = "8.8.8.8";

fn discord_config() -> DiscordConfig {
    DiscordConfig {
        guild_id: "guild".to_string(),
        bot_token: "bot".to_string(),
        client_id: "client".to_string(),
        client_secret: "secret".to_string(),
        qualifying_role_ids: vec!["role-a".to_string()],
    }
}

fn qualifying() -> MemberSource {
    MemberSource::Fixed(FixedMemberSource::new(vec!["role-a".to_string()]))
}

fn lapsed() -> MemberSource {
    MemberSource::Fixed(FixedMemberSource::new(vec!["role-z".to_string()]))
}

struct Enrolled {
    conn: Arc<DatabaseConnection>,
    registry: Arc<RegistryService>,
    name: String,
}

impl Enrolled {
    fn fqdn(&self) -> String {
        format!("{}.{ZONE}", self.name)
    }

    async fn row(&self) -> registration::Model {
        registration::Entity::find_by_id("node-a")
            .one(self.conn.as_ref())
            .await
            .expect("reads")
            .expect("registered")
    }

    async fn sql(&self, statement: &str) {
        self.conn
            .execute_unprepared(statement)
            .await
            .expect("updates");
    }

    async fn check_in(&self, address: Option<&str>, port: u16) -> String {
        let address = address.map(|a| a.parse::<IpAddr>().expect("an address"));
        self.registry
            .record_check_in("node-a", address, port)
            .await
            .expect("checks in")
            .nonce
    }
}

async fn enrolled() -> Enrolled {
    let conn = Arc::new(Db::connect("sqlite::memory:").await.expect("connects"));
    let registry = RegistryService::new_shared(conn.clone(), discord_config(), qualifying());
    let token = registry.issue_token("member-1").await.expect("issues");
    let name = registry.redeem(&token, "node-a").await.expect("redeems");
    Enrolled {
        conn,
        registry,
        name,
    }
}

fn zone(conn: Arc<DatabaseConnection>, api: Arc<RecordingApi>) -> Arc<ZoneWriter> {
    Arc::new(ZoneWriter::new(
        conn,
        CloudflareApi::Recording(api),
        ZONE.to_string(),
    ))
}

fn scheduler_with(
    conn: Arc<DatabaseConnection>,
    registry: Arc<RegistryService>,
    zone: Arc<ZoneWriter>,
    members: MemberSource,
    probe: AddressProbe,
) -> Arc<DailyScheduler> {
    let checker = ValidationChecker::new_shared(conn.clone(), registry.clone(), zone.clone());
    let verifier = AddressVerifier::new_shared(probe, zone.clone());
    DailyScheduler::new_shared(
        conn.clone(),
        checker,
        verifier,
        registry,
        zone,
        ClaimService::new_shared(conn),
        members,
        discord_config(),
    )
}

fn scheduler(
    conn: Arc<DatabaseConnection>,
    registry: Arc<RegistryService>,
    members: MemberSource,
) -> Arc<DailyScheduler> {
    let zone = zone(conn.clone(), Arc::new(RecordingApi::new()));
    scheduler_with(conn, registry, zone, members, AddressProbe::Fixed(true))
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

#[tokio::test]
async fn a_member_who_no_longer_qualifies_is_revoked_without_contacting_their_server() {
    let node = enrolled().await;
    node.check_in(None, 443).await;
    let scheduler = scheduler(node.conn.clone(), node.registry.clone(), lapsed());

    for _ in 0..ValidationChecker::FAILURE_THRESHOLD {
        scheduler.run_once().await.expect("a pass runs");
    }

    assert_eq!(node.row().await.state, "revoked");
    assert_eq!(node.registry.name_for("node-a").await.expect("lookup"), None);
}

#[tokio::test]
async fn a_pass_over_an_empty_registry_evaluates_nothing() {
    let conn = Arc::new(Db::connect("sqlite::memory:").await.expect("connects"));
    let registry = RegistryService::new_shared(
        conn.clone(),
        discord_config(),
        MemberSource::Fixed(FixedMemberSource::absent()),
    );
    let scheduler = scheduler(
        conn,
        registry,
        MemberSource::Fixed(FixedMemberSource::absent()),
    );

    assert_eq!(scheduler.run_once().await.expect("a pass runs"), 0);
}

#[tokio::test]
async fn a_pass_reports_how_many_registrations_it_evaluated() {
    let node = enrolled().await;
    let scheduler = scheduler(node.conn.clone(), node.registry.clone(), qualifying());

    assert_eq!(scheduler.run_once().await.expect("a pass runs"), 1);
}

#[tokio::test]
async fn a_revoked_registration_is_still_evaluated() {
    let node = enrolled().await;
    let scheduler = scheduler(node.conn.clone(), node.registry.clone(), lapsed());

    for _ in 0..ValidationChecker::FAILURE_THRESHOLD {
        scheduler.run_once().await.expect("a pass runs");
    }
    assert_eq!(node.row().await.state, "revoked");

    assert_eq!(scheduler.run_once().await.expect("a pass runs"), 1);
}

#[tokio::test]
async fn a_registration_is_due_a_day_after_its_last_check() {
    let node = enrolled().await;
    let scheduler = scheduler(node.conn.clone(), node.registry.clone(), qualifying());
    scheduler.run_once().await.expect("a pass runs");

    let day = DailyScheduler::INTERVAL.as_secs() as i64;
    assert_eq!(scheduler.run_due(now()).await.expect("a pass runs"), 0);
    assert_eq!(
        scheduler.run_due(now() + day).await.expect("a pass runs"),
        1
    );
}

#[tokio::test]
async fn ticks_within_the_interval_do_not_accumulate_failures() {
    let node = enrolled().await;
    let scheduler = scheduler(node.conn.clone(), node.registry.clone(), lapsed());
    let day = DailyScheduler::INTERVAL.as_secs() as i64;

    scheduler.run_due(now() + day).await.expect("a pass runs");
    for _ in 0..(ValidationChecker::FAILURE_THRESHOLD * 2) {
        scheduler.run_due(now()).await.expect("a pass runs");
    }

    assert_eq!(node.row().await.validation_failures, 1);
    assert!(
        node.registry
            .name_for("node-a")
            .await
            .expect("lookup")
            .is_some(),
        "one failed check must not become a revocation"
    );
}

#[tokio::test]
async fn identity_passes_on_a_check_in_within_a_day() {
    let node = enrolled().await;
    node.check_in(None, 443).await;
    let scheduler = scheduler(node.conn.clone(), node.registry.clone(), qualifying());

    scheduler.run_once().await.expect("a pass runs");

    assert_eq!(node.row().await.validation_failures, 0);
}

#[tokio::test]
async fn identity_fails_on_a_check_in_older_than_a_day() {
    let node = enrolled().await;
    node.check_in(None, 443).await;
    node.sql(&format!(
        "UPDATE registration SET checked_in_at = {}",
        now() - 86_401
    ))
    .await;
    let scheduler = scheduler(node.conn.clone(), node.registry.clone(), qualifying());

    scheduler.run_once().await.expect("a pass runs");

    assert_eq!(node.row().await.validation_failures, 1);
}

#[tokio::test]
async fn a_fresh_scheduler_reads_the_stored_check_in() {
    let node = enrolled().await;
    let first = scheduler(node.conn.clone(), node.registry.clone(), qualifying());
    node.check_in(None, 443).await;
    drop(first);

    let restarted = scheduler(node.conn.clone(), node.registry.clone(), qualifying());
    restarted.run_once().await.expect("a pass runs");

    assert_eq!(node.row().await.validation_failures, 0);
}

async fn probe_once(node: &Enrolled, probe: AddressProbe) -> i32 {
    let zone = zone(node.conn.clone(), Arc::new(RecordingApi::new()));
    let scheduler = scheduler_with(
        node.conn.clone(),
        node.registry.clone(),
        zone,
        qualifying(),
        probe,
    );
    scheduler.run_once().await.expect("a pass runs");
    node.row().await.validation_failures
}

#[tokio::test]
async fn the_probe_uses_the_declared_port_and_the_check_in_nonce() {
    for (declared, failures) in [(8443u16, 0), (443u16, 1)] {
        let node = enrolled().await;
        let nonce = node.check_in(Some(PUBLIC), declared).await;

        let probe = AddressProbe::ForPort(node.fqdn(), 8443, nonce);
        assert_eq!(
            probe_once(&node, probe).await,
            failures,
            "declared port {declared}"
        );
    }
}

#[tokio::test]
async fn a_nonce_from_an_earlier_check_in_fails_the_probe() {
    let node = enrolled().await;
    let stale = node.check_in(Some(PUBLIC), 8443).await;
    node.check_in(Some(PUBLIC), 8443).await;

    let probe = AddressProbe::ForPort(node.fqdn(), 8443, stale);
    assert_eq!(probe_once(&node, probe).await, 1);
}

#[tokio::test]
async fn a_registration_without_a_declared_port_is_probed_on_443() {
    let node = enrolled().await;
    let nonce = node.check_in(Some(PUBLIC), 8443).await;
    node.sql("UPDATE registration SET declared_port = NULL").await;

    let probe = AddressProbe::ForPort(node.fqdn(), 443, nonce);
    assert_eq!(probe_once(&node, probe).await, 0);
}

#[tokio::test]
async fn a_revoked_registration_with_lapsed_entitlement_does_not_recover_while_discord_is_down() {
    let node = enrolled().await;
    node.registry.revoke("node-a").await.expect("revokes");
    node.sql("UPDATE registration SET entitlement_ok = 0").await;
    node.check_in(None, 443).await;
    let scheduler = scheduler(
        node.conn.clone(),
        node.registry.clone(),
        MemberSource::Fixed(FixedMemberSource::unreachable()),
    );

    scheduler.run_once().await.expect("a pass runs");

    assert_eq!(node.row().await.state, "revoked");
}

#[tokio::test]
async fn a_revoked_registration_with_stored_entitlement_recovers_while_discord_is_down() {
    let node = enrolled().await;
    node.registry.revoke("node-a").await.expect("revokes");
    node.check_in(None, 443).await;
    let scheduler = scheduler(
        node.conn.clone(),
        node.registry.clone(),
        MemberSource::Fixed(FixedMemberSource::unreachable()),
    );

    scheduler.run_once().await.expect("a pass runs");

    let row = node.row().await;
    assert!(row.entitlement_ok);
    assert_eq!(row.state, "active");
}

#[tokio::test]
async fn a_tick_withdraws_an_address_a_revoked_registration_still_has() {
    let node = enrolled().await;
    let api = Arc::new(RecordingApi::new());
    let zone = zone(node.conn.clone(), api.clone());
    node.registry.revoke("node-a").await.expect("revokes");
    zone.publish_address(&node.name, PUBLIC.parse().expect("an address"))
        .await
        .expect("publishes");
    let scheduler = scheduler_with(
        node.conn.clone(),
        node.registry.clone(),
        zone.clone(),
        qualifying(),
        AddressProbe::Fixed(false),
    );

    scheduler.tick(now()).await;

    assert!(!zone.has_address_record(&node.name).await.expect("reads"));
    assert!(api.live_ids().is_empty());
}

#[tokio::test]
async fn a_registration_in_an_unknown_state_has_its_records_withdrawn() {
    let node = enrolled().await;
    let api = Arc::new(RecordingApi::new());
    let zone = zone(node.conn.clone(), api.clone());
    zone.publish_address(&node.name, PUBLIC.parse().expect("an address"))
        .await
        .expect("publishes");
    node.sql("UPDATE registration SET state = 'blacklisted'").await;
    let scheduler = scheduler_with(
        node.conn.clone(),
        node.registry.clone(),
        zone,
        qualifying(),
        AddressProbe::Fixed(false),
    );

    scheduler.tick(now()).await;

    assert!(api.live_ids().is_empty());
}

#[tokio::test]
async fn a_suspended_registration_is_not_evaluated_and_its_records_are_withdrawn() {
    let node = enrolled().await;
    let api = Arc::new(RecordingApi::new());
    let zone = zone(node.conn.clone(), api.clone());
    zone.publish_address(&node.name, PUBLIC.parse().expect("an address"))
        .await
        .expect("publishes");
    node.sql("UPDATE registration SET state = 'suspended', entitlement_checked_at = NULL")
        .await;
    let scheduler = scheduler_with(
        node.conn.clone(),
        node.registry.clone(),
        zone,
        lapsed(),
        AddressProbe::Fixed(false),
    );

    scheduler.tick(now()).await;

    assert_eq!(node.row().await.validation_failures, 0);
    assert!(api.live_ids().is_empty());
}
