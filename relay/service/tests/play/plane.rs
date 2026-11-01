use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use bvc_relay_service::db::Db;
use bvc_relay_service::play::{
    LocalSigner, PlayAddressProbe, PlayIssuanceService, PlayNameService, PlayPlane, PlaySigner,
    PlayZone,
};
use bvc_relay_service::validation::AddressProbe;
use sea_orm::{ConnectionTrait, DatabaseConnection};

use crate::harness::PlayFixture;

const NODE: &str = "node-a";
const PORT: u16 = 19132;

struct Setup {
    conn: Arc<DatabaseConnection>,
    names: Arc<PlayNameService>,
    zone: Arc<PlayZone>,
    plane: Arc<PlayPlane>,
}

async fn setup(probe_succeeds: bool) -> Setup {
    setup_with(probe_succeeds, 10).await
}

async fn setup_with(probe_succeeds: bool, delay_millis: u64) -> Setup {
    let conn = Arc::new(Db::connect("sqlite::memory:").await.expect("connects"));
    let config = PlayFixture::config();
    let names = PlayNameService::new_shared(conn.clone(), config.clone());
    let zone = PlayZone::new_shared(config.zone.clone(), config.dns.clone());
    let issuance =
        PlayIssuanceService::new_shared(names.clone(), PlaySigner::Local(LocalSigner::new()));
    let probe = PlayAddressProbe::new(
        AddressProbe::Fixed(probe_succeeds),
        vec![
            Duration::from_millis(delay_millis),
            Duration::from_millis(delay_millis),
        ],
    );
    let plane = PlayPlane::new_shared(names.clone(), issuance, zone.clone(), probe);

    Setup {
        conn,
        names,
        zone,
        plane,
    }
}

fn address(text: &str) -> IpAddr {
    text.parse().expect("an IP address")
}

// The probe runs in the background, so a test waits for its outcome.
async fn eventually(mut condition: impl FnMut() -> bool) -> bool {
    for _ in 0..100 {
        if condition() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    false
}

async fn check_in(setup: &Setup, candidate: IpAddr) {
    let record = setup
        .plane
        .check_in(NODE, candidate, PORT)
        .await
        .expect("checks in");
    setup
        .plane
        .verify_and_publish(NODE.to_string(), record, candidate, PORT);
}

// Publishes an address the way an earlier successful check-in would have.
async fn publish_earlier(setup: &Setup, label: &str, earlier: IpAddr) {
    let record = setup
        .names
        .record_check_in(NODE, earlier, PORT)
        .await
        .expect("checks in");
    let serial = setup
        .names
        .mark_published(NODE, earlier, &record.nonce)
        .await
        .expect("publishes")
        .expect("current");
    setup.zone.publish(label, earlier, serial);
}

#[tokio::test]
async fn a_check_in_publishes_only_after_the_probe_succeeds() {
    let failing = setup(false).await;
    let label = failing.names.claim(NODE, address("203.0.113.7")).await.expect("claims");
    check_in(&failing, address("198.51.100.4")).await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert_eq!(failing.zone.lookup(&label), None);
    assert!(failing.names.published().await.expect("reads").is_empty());

    let passing = setup(true).await;
    let label = passing.names.claim(NODE, address("203.0.113.7")).await.expect("claims");
    check_in(&passing, address("198.51.100.4")).await;
    assert!(eventually(|| passing.zone.lookup(&label).is_some()).await);
    assert_eq!(passing.zone.lookup(&label), Some(address("198.51.100.4")));
    assert_eq!(passing.names.published().await.expect("reads").len(), 1);
}

// The server moved and the new address does not answer. The old record points at an
// address nobody has proved since the move, so it must not stay.
#[tokio::test]
async fn a_changed_address_whose_probe_fails_withdraws_the_old_record() {
    let setup = setup(false).await;
    let label = setup.names.claim(NODE, address("203.0.113.7")).await.expect("claims");
    publish_earlier(&setup, &label, address("198.51.100.4")).await;
    assert!(setup.zone.lookup(&label).is_some());

    check_in(&setup, address("198.51.100.9")).await;

    assert!(eventually(|| setup.zone.lookup(&label).is_none()).await);
    assert!(setup.names.published().await.expect("reads").is_empty());
}

#[tokio::test]
async fn an_unchanged_address_whose_probe_fails_keeps_the_record() {
    let setup = setup(false).await;
    let label = setup.names.claim(NODE, address("203.0.113.7")).await.expect("claims");
    publish_earlier(&setup, &label, address("198.51.100.4")).await;

    check_in(&setup, address("198.51.100.4")).await;
    tokio::time::sleep(Duration::from_millis(400)).await;

    assert_eq!(setup.zone.lookup(&label), Some(address("198.51.100.4")));
}

// The server moved to B, then went back to A before B's probe finished. B's probe would
// fail and withdraw the record; it must not, because the newer check-in replaced it.
#[tokio::test]
async fn a_newer_check_in_cancels_the_earlier_probe_and_keeps_the_record() {
    let setup = setup_with(false, 200).await;
    let label = setup.names.claim(NODE, address("203.0.113.7")).await.expect("claims");
    publish_earlier(&setup, &label, address("198.51.100.4")).await;

    check_in(&setup, address("198.51.100.9")).await;
    check_in(&setup, address("198.51.100.4")).await;
    tokio::time::sleep(Duration::from_millis(900)).await;

    assert_eq!(setup.zone.lookup(&label), Some(address("198.51.100.4")));
    assert_eq!(setup.names.published().await.expect("reads").len(), 1);
}

#[tokio::test]
async fn a_throttled_check_in_starts_no_probe() {
    let setup = setup(true).await;
    let label = setup.names.claim(NODE, address("203.0.113.7")).await.expect("claims");
    let mut throttled = setup
        .names
        .record_check_in(NODE, address("198.51.100.4"), PORT)
        .await
        .expect("checks in");
    throttled.throttled = true;

    setup
        .plane
        .verify_and_publish(NODE.to_string(), throttled, address("198.51.100.4"), PORT);
    tokio::time::sleep(Duration::from_millis(300)).await;

    assert_eq!(setup.zone.lookup(&label), None);
}

#[tokio::test]
async fn a_released_name_leaves_the_zone() {
    let setup = setup(true).await;
    let label = setup.names.claim(NODE, address("203.0.113.7")).await.expect("claims");
    publish_earlier(&setup, &label, address("198.51.100.4")).await;
    setup
        .conn
        .execute_unprepared("UPDATE play_name SET checked_in_at = 1 WHERE node_id = 'node-a'")
        .await
        .expect("ages the check-in");

    setup.plane.release_stale().await.expect("sweeps");

    assert_eq!(setup.zone.lookup(&label), None);
    assert_eq!(setup.names.name_for(NODE).await.expect("reads"), None);
}

// Two nodes publishing at once take consecutive serials, and the zone drops a change
// whose serial is behind its own. Both records must survive.
#[tokio::test]
async fn simultaneous_publications_all_reach_the_zone() {
    let setup = setup(true).await;
    let mut labels = Vec::new();
    let mut pending = Vec::new();
    for index in 0..8 {
        let node = format!("node-{index}");
        let observed = address(&format!("203.0.113.{}", 10 + index));
        let label = setup.names.claim(&node, observed).await.expect("claims");
        let candidate = address(&format!("198.51.100.{}", 10 + index));
        let record = setup
            .plane
            .check_in(&node, candidate, PORT)
            .await
            .expect("checks in");
        pending.push((node, record, candidate));
        labels.push(label);
    }
    for (node, record, candidate) in pending {
        setup.plane.verify_and_publish(node, record, candidate, PORT);
    }

    assert!(eventually(|| labels.iter().all(|l| setup.zone.lookup(l).is_some())).await);
}
