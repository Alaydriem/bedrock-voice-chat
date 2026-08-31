use std::sync::Arc;

use bvc_relay_service::config::DiscordConfig;
use bvc_relay_service::db::Db;
use bvc_relay_service::discord::{FixedMemberSource, MemberSource};
use bvc_relay_service::dns::{CloudflareApi, RecordingApi, ZoneWriter};
use bvc_relay_service::enroll::EnrollSessions;
use bvc_relay_service::registry::RegistryService;
use bvc_relay_service::scheduler::DailyScheduler;
use bvc_relay_service::validation::{AddressProbe, ValidationChecker};

fn discord_config() -> DiscordConfig {
    DiscordConfig {
        guild_id: "guild".to_string(),
        bot_token: "bot".to_string(),
        client_id: "client".to_string(),
        client_secret: "secret".to_string(),
        qualifying_role_ids: vec!["role-a".to_string()],
    }
}

async fn enrolled() -> (Arc<sea_orm::DatabaseConnection>, Arc<RegistryService>) {
    let conn = Arc::new(Db::connect("sqlite::memory:").await.expect("connects"));
    let registry = RegistryService::new_shared(
        conn.clone(),
        discord_config(),
        MemberSource::Fixed(FixedMemberSource::new(vec!["role-a".to_string()])),
    );
    let token = registry.issue_token("member-1").await.expect("issues");
    registry.redeem(&token, "node-a").await.expect("redeems");
    (conn, registry)
}

fn scheduler(
    conn: Arc<sea_orm::DatabaseConnection>,
    registry: Arc<RegistryService>,
    members: MemberSource,
) -> Arc<DailyScheduler> {
    let zone = Arc::new(ZoneWriter::new(
        conn.clone(),
        CloudflareApi::Recording(Arc::new(RecordingApi::new())),
        "bedrockvc.stream".to_string(),
    ));
    let checker = ValidationChecker::new_shared(conn.clone(), registry, zone);
    DailyScheduler::new_shared(
        conn,
        checker,
        EnrollSessions::new_shared(),
        members,
        discord_config(),
        AddressProbe::Fixed(true),
    )
}

// The entitlement half runs entirely on the relay. A member whose roles no longer
// qualify loses their registration without the server being contacted, which is what
// makes an entitlement lapse unable to break a running server.
#[tokio::test]
async fn a_member_who_no_longer_qualifies_is_suspended_without_contacting_their_server() {
    let (conn, registry) = enrolled().await;
    let scheduler = scheduler(
        conn,
        registry.clone(),
        MemberSource::Fixed(FixedMemberSource::new(vec!["role-z".to_string()])),
    );

    for _ in 0..ValidationChecker::FAILURE_THRESHOLD {
        scheduler.run_once().await.expect("a pass runs");
    }

    assert_eq!(registry.name_for("node-a").await.expect("lookup"), None);
}

// A pass over an empty registry does nothing and reports nothing evaluated, rather
// than erroring on a table with no rows.
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

// A pass counts the registrations it evaluated, so an operator watching the metric
// can tell a pass that ran and found nothing from one that never ran.
#[tokio::test]
async fn a_pass_reports_how_many_registrations_it_evaluated() {
    let (conn, registry) = enrolled().await;
    let scheduler = scheduler(
        conn,
        registry,
        MemberSource::Fixed(FixedMemberSource::new(vec!["role-a".to_string()])),
    );

    assert_eq!(scheduler.run_once().await.expect("a pass runs"), 1);
}

// A suspended registration drops out of later passes. Continuing to evaluate it
// would keep incrementing a counter nothing reads and keep calling Discord for a
// member the relay no longer serves.
#[tokio::test]
async fn a_suspended_registration_is_not_evaluated_again() {
    let (conn, registry) = enrolled().await;
    let scheduler = scheduler(
        conn,
        registry,
        MemberSource::Fixed(FixedMemberSource::new(vec!["role-z".to_string()])),
    );

    for _ in 0..ValidationChecker::FAILURE_THRESHOLD {
        scheduler.run_once().await.expect("a pass runs");
    }

    assert_eq!(scheduler.run_once().await.expect("a pass runs"), 0);
}
