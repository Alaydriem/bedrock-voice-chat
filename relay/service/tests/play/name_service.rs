use std::net::IpAddr;
use std::sync::Arc;

use bvc_relay_service::config::PlayConfig;
use bvc_relay_service::db::Db;
use bvc_relay_service::entity::play_name;
use bvc_relay_service::naming::NameGenerator;
use bvc_relay_service::play::{PlayError, PlayNameService};
use sea_orm::{ConnectionTrait, DatabaseConnection, EntityTrait, PaginatorTrait};

use crate::harness::PlayFixture;

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

async fn database() -> Arc<DatabaseConnection> {
    Arc::new(Db::connect("sqlite::memory:").await.expect("connects"))
}

fn address(text: &str) -> IpAddr {
    text.parse().expect("an IP address")
}

// A check-in followed by the publication it proves.
async fn publish(service: &PlayNameService, node: &str, text: &str) -> u32 {
    let check_in = service
        .record_check_in(node, address(text), 19132)
        .await
        .expect("checks in");

    service
        .mark_published(node, address(text), &check_in.nonce)
        .await
        .expect("publishes")
        .expect("the check-in is current")
}

async fn service_with(config: PlayConfig) -> (Arc<DatabaseConnection>, PlayNameService) {
    let conn = database().await;
    let service = PlayNameService::new(conn.clone(), config);
    (conn, service)
}

async fn service() -> (Arc<DatabaseConnection>, PlayNameService) {
    service_with(PlayFixture::config()).await
}

#[tokio::test]
async fn a_second_claim_returns_the_same_name() {
    let (_conn, service) = service().await;

    let first = service.claim("node-a", address("203.0.113.7")).await.expect("claims");
    let second = service.claim("node-a", address("203.0.113.7")).await.expect("claims");

    assert_eq!(first, second);
}

// A fast restart, or a retry that overlaps a slow reply, claims twice at once. Both
// must be told the same name and only one row may exist.
#[tokio::test]
async fn concurrent_claims_from_one_node_create_one_row() {
    let (conn, service) = service().await;
    let observed = address("203.0.113.7");

    let (first, second) = tokio::join!(
        service.claim("node-a", observed),
        service.claim("node-a", observed)
    );

    assert_eq!(first.expect("claims"), second.expect("claims"));
    assert_eq!(
        play_name::Entity::find().count(conn.as_ref()).await.expect("counts"),
        1
    );
}

#[tokio::test]
async fn the_fourth_claim_from_one_address_in_a_day_is_refused() {
    let (_conn, service) = service().await;
    let observed = address("203.0.113.7");

    for node in ["node-a", "node-b", "node-c"] {
        service.claim(node, observed).await.expect("within the limit");
    }

    assert!(matches!(
        service.claim("node-d", observed).await,
        Err(PlayError::RateLimited)
    ));
}

#[tokio::test]
async fn the_global_hourly_limit_refuses() {
    let mut config = PlayFixture::config();
    config.claims_per_hour = 2;
    let (_conn, service) = service_with(config).await;

    service.claim("node-a", address("203.0.113.1")).await.expect("claims");
    service.claim("node-b", address("203.0.113.2")).await.expect("claims");

    assert!(matches!(
        service.claim("node-c", address("203.0.113.3")).await,
        Err(PlayError::RateLimited)
    ));
}

#[tokio::test]
async fn a_name_without_check_in_for_thirty_days_is_released() {
    let (conn, service) = service().await;
    let label = service.claim("node-a", address("203.0.113.7")).await.expect("claims");
    conn.execute_unprepared(&format!(
        "UPDATE play_name SET checked_in_at = {} WHERE node_id = 'node-a'",
        now() - 31 * 86_400
    ))
    .await
    .expect("ages the check-in");

    let (released, _serial) = service.release_stale().await.expect("sweeps");

    assert_eq!(released, vec![label.clone()]);
    assert_eq!(service.name_for("node-a").await.expect("reads"), None);
    assert!(
        !NameGenerator::is_available(conn.as_ref(), &label)
            .await
            .expect("checks")
    );
}

#[tokio::test]
async fn a_recent_check_in_keeps_the_name() {
    let (_conn, service) = service().await;
    let label = service.claim("node-a", address("203.0.113.7")).await.expect("claims");
    service
        .record_check_in("node-a", address("203.0.113.7"), 19132)
        .await
        .expect("checks in");

    let (released, _serial) = service.release_stale().await.expect("sweeps");

    assert!(released.is_empty());
    assert_eq!(service.name_for("node-a").await.expect("reads"), Some(label));
}

#[tokio::test]
async fn play_names_and_assigned_names_never_share_a_label() {
    let (conn, service) = service().await;
    conn.execute_unprepared(
        "INSERT INTO registration (node_id, name, discord_user_id, state, entitlement_ok, \
         validation_failures, created_at) \
         VALUES ('node-r', 'brave-otter-mesa', 'member-1', 'active', 1, 0, 0)",
    )
    .await
    .expect("a registration inserts");
    let label = service.claim("node-a", address("203.0.113.7")).await.expect("claims");

    assert!(
        !NameGenerator::is_available(conn.as_ref(), "brave-otter-mesa")
            .await
            .expect("checks")
    );
    assert!(
        !NameGenerator::is_available(conn.as_ref(), &label)
            .await
            .expect("checks")
    );
}

// A secondary ignores a zone whose serial is not larger than the one it holds, so a
// serial that restarted from the clock alone could land below one already handed out.
#[tokio::test]
async fn the_serial_never_goes_backwards_across_a_restart() {
    let (conn, service) = service().await;
    service.claim("node-a", address("203.0.113.7")).await.expect("claims");
    let ahead = now() + 1000;
    conn.execute_unprepared(&format!("UPDATE play_zone_state SET serial = {ahead}"))
        .await
        .expect("moves the serial ahead");

    let restarted = PlayNameService::new(conn.clone(), PlayFixture::config());
    let serial = publish(&restarted, "node-a", "198.51.100.4").await;

    assert_eq!(i64::from(serial), ahead + 1);
}

#[tokio::test]
async fn publishing_the_same_address_again_keeps_the_serial() {
    let (_conn, service) = service().await;
    service.claim("node-a", address("203.0.113.7")).await.expect("claims");

    let first = publish(&service, "node-a", "198.51.100.4").await;
    let second = publish(&service, "node-a", "198.51.100.4").await;

    assert_eq!(first, second);
    assert_eq!(service.serial().await.expect("reads"), first);
}

#[tokio::test]
async fn a_check_in_reports_whether_the_address_differs_from_the_published_one() {
    let (_conn, service) = service().await;
    service.claim("node-a", address("203.0.113.7")).await.expect("claims");
    publish(&service, "node-a", "198.51.100.4").await;

    let same = service
        .record_check_in("node-a", address("198.51.100.4"), 19132)
        .await
        .expect("checks in");
    let moved = service
        .record_check_in("node-a", address("198.51.100.9"), 19132)
        .await
        .expect("checks in");

    assert!(!same.address_changed);
    assert!(moved.address_changed);
}

#[tokio::test]
async fn a_withdrawn_name_leaves_the_published_set_and_advances_the_serial() {
    let (_conn, service) = service().await;
    let label = service.claim("node-a", address("203.0.113.7")).await.expect("claims");
    let published = publish(&service, "node-a", "198.51.100.4").await;
    assert_eq!(
        service.published().await.expect("reads"),
        vec![(label, address("198.51.100.4"))]
    );

    let moved = service
        .record_check_in("node-a", address("198.51.100.9"), 19132)
        .await
        .expect("checks in");
    let withdrawn = service
        .mark_withdrawn("node-a", &moved.nonce)
        .await
        .expect("withdraws")
        .expect("the check-in is current");

    assert!(withdrawn > published);
    assert!(service.published().await.expect("reads").is_empty());
}

#[tokio::test]
async fn a_node_without_a_name_cannot_check_in() {
    let (_conn, service) = service().await;

    assert!(matches!(
        service
            .record_check_in("node-a", address("203.0.113.7"), 19132)
            .await,
        Err(PlayError::NoPlayName)
    ));
}

#[tokio::test]
async fn a_node_that_released_its_name_is_given_a_new_one() {
    let (conn, service) = service().await;
    let old = service.claim("node-a", address("203.0.113.7")).await.expect("claims");
    conn.execute_unprepared(&format!(
        "UPDATE play_name SET checked_in_at = {} WHERE node_id = 'node-a'",
        now() - 31 * 86_400
    ))
    .await
    .expect("ages the check-in");
    service.release_stale().await.expect("sweeps");

    let fresh = service.claim("node-a", address("203.0.113.7")).await.expect("claims");

    assert_ne!(fresh, old);
    assert_eq!(service.name_for("node-a").await.expect("reads"), Some(fresh));
}

// Play labels carry a fourth, numeric part, so they come from a space of their own and
// can never equal an assigned name.
#[tokio::test]
async fn a_claimed_label_has_four_parts_ending_in_four_digits() {
    let (_conn, service) = service().await;

    let label = service.claim("node-a", address("203.0.113.7")).await.expect("claims");

    let parts: Vec<&str> = label.split('-').collect();
    assert_eq!(parts.len(), 4, "{label}");
    assert_eq!(parts[3].len(), 4);
    assert!(parts[3].bytes().all(|b| b.is_ascii_digit()));
}

#[tokio::test]
async fn assigned_names_keep_their_three_part_shape() {
    let (conn, _service) = service().await;

    let name = NameGenerator::new().assign(conn.as_ref()).await.expect("assigns");

    assert_eq!(name.split('-').count(), 3, "{name}");
}

// Fifty connections at once from one address, each with its own key, must not all pass
// a limit that counts rows the others have not inserted yet.
#[tokio::test]
async fn concurrent_claims_from_distinct_nodes_at_one_address_yield_exactly_the_limit() {
    let conn = database().await;
    let service = PlayNameService::new_shared(conn, PlayFixture::config());

    let attempts = (0..12).map(|index| {
        let service = service.clone();
        tokio::spawn(async move {
            service
                .claim(&format!("node-{index}"), address("203.0.113.7"))
                .await
        })
    });
    let results = futures_util::future::join_all(attempts).await;

    let granted = results
        .iter()
        .filter(|r| matches!(r, Ok(Ok(_))))
        .count();
    let refused = results
        .iter()
        .filter(|r| matches!(r, Ok(Err(PlayError::RateLimited))))
        .count();
    assert_eq!(granted, 3);
    assert_eq!(refused, 9);
}

#[tokio::test]
async fn addresses_in_one_ipv6_64_share_a_bucket() {
    let (_conn, service) = service().await;

    for (node, source) in [
        ("node-a", "2001:db8:1:2::1"),
        ("node-b", "2001:db8:1:2:ffff::2"),
        ("node-c", "2001:db8:1:2::3"),
    ] {
        service.claim(node, address(source)).await.expect("within the limit");
    }

    assert!(matches!(
        service.claim("node-d", address("2001:db8:1:2:abcd::4")).await,
        Err(PlayError::RateLimited)
    ));
    service
        .claim("node-e", address("2001:db8:1:3::1"))
        .await
        .expect("another /64 has its own allowance");
}

#[tokio::test]
async fn an_ipv4_mapped_address_shares_the_bucket_of_its_ipv4_address() {
    let (_conn, service) = service().await;
    service.claim("node-a", address("203.0.113.7")).await.expect("claims");
    service.claim("node-b", address("203.0.113.7")).await.expect("claims");
    service.claim("node-c", address("203.0.113.7")).await.expect("claims");

    assert!(matches!(
        service.claim("node-d", address("::ffff:203.0.113.7")).await,
        Err(PlayError::RateLimited)
    ));
}

// A check-in that was replaced while its probe ran must not change the newer record:
// the earlier probe's late failure would otherwise delete an address just proven.
#[tokio::test]
async fn a_superseded_check_in_changes_nothing() {
    let (_conn, service) = service().await;
    service.claim("node-a", address("203.0.113.7")).await.expect("claims");
    let first = service
        .record_check_in("node-a", address("198.51.100.9"), 19132)
        .await
        .expect("checks in");
    let second = service
        .record_check_in("node-a", address("198.51.100.9"), 19132)
        .await
        .expect("checks in");
    service
        .mark_published("node-a", address("198.51.100.9"), &second.nonce)
        .await
        .expect("publishes")
        .expect("current");

    let withdrawn = service
        .mark_withdrawn("node-a", &first.nonce)
        .await
        .expect("answers");
    let republished = service
        .mark_published("node-a", address("198.51.100.9"), &first.nonce)
        .await
        .expect("answers");

    assert_eq!(withdrawn, None);
    assert_eq!(republished, None);
    assert_eq!(service.published().await.expect("reads").len(), 1);
}

#[tokio::test]
async fn a_second_check_in_inside_the_window_repeats_the_nonce_and_starts_no_probe() {
    let mut config = PlayFixture::config();
    config.min_check_in_interval_secs = 60;
    let (_conn, service) = service_with(config).await;
    service.claim("node-a", address("203.0.113.7")).await.expect("claims");

    let first = service
        .record_check_in("node-a", address("198.51.100.4"), 19132)
        .await
        .expect("checks in");
    let second = service
        .record_check_in("node-a", address("198.51.100.9"), 19132)
        .await
        .expect("checks in");

    assert!(!first.throttled);
    assert!(second.throttled);
    assert_eq!(first.nonce, second.nonce);
}

#[tokio::test]
async fn a_check_in_naming_a_non_global_address_is_refused() {
    let mut config = PlayFixture::config();
    config.allow_private_probe_targets = false;
    let (_conn, service) = service_with(config).await;
    service.claim("node-a", address("203.0.113.7")).await.expect("claims");

    for target in [
        "127.0.0.1",
        "10.0.0.5",
        "172.18.0.2",
        "192.168.1.1",
        "100.64.0.1",
        "169.254.1.1",
        "0.0.0.0",
        "224.0.0.1",
        "203.0.113.9",
        "::1",
        "fd00::1",
        "fe80::1",
        "ff02::1",
        "2001:db8::1",
        "::ffff:10.0.0.1",
    ] {
        assert!(
            matches!(
                service.record_check_in("node-a", address(target), 19132).await,
                Err(PlayError::AddressRefused(_))
            ),
            "{target}"
        );
    }

    service
        .record_check_in("node-a", address("8.8.8.8"), 19132)
        .await
        .expect("a global address is accepted");
}

#[tokio::test]
async fn certificate_requests_beyond_the_daily_allowance_are_refused() {
    let mut config = PlayFixture::config();
    config.issuances_per_node_per_day = 2;
    let (_conn, service) = service_with(config).await;

    service.admit_issuance("node-a").await.expect("first");
    service.admit_issuance("node-a").await.expect("second");

    assert!(matches!(
        service.admit_issuance("node-a").await,
        Err(PlayError::RateLimited)
    ));
    service
        .admit_issuance("node-b")
        .await
        .expect("another node has its own allowance");
}

#[tokio::test]
async fn fqdn_and_label_of_are_inverses_inside_the_zone() {
    let (_conn, service) = service().await;

    let fqdn = service.fqdn("brave-otter-mesa");

    assert_eq!(fqdn, "brave-otter-mesa.play.bedrockvc.stream");
    assert_eq!(service.label_of(&fqdn), Some("brave-otter-mesa".to_string()));
    assert_eq!(service.label_of("brave-otter-mesa.bedrockvc.stream"), None);
    assert_eq!(service.label_of("a.b.play.bedrockvc.stream"), None);
}
