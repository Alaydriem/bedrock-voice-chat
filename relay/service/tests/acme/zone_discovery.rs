use bvc_relay_service::acme::CloudflareDns;
use serde_json::json;

use crate::harness::{MockApi, MockRoute};

const HOSTNAME: &str = "registry.bedrockvoicechat.com";
const APEX: &str = "bedrockvoicechat.com";

fn empty() -> serde_json::Value {
    json!({ "success": true, "result": [] })
}

fn zone(id: &str) -> serde_json::Value {
    json!({ "success": true, "result": [{ "id": id }] })
}

// The walk exists so one token covering both zones needs no zone id in config. The
// first candidate is the full hostname, which is not a zone — the answer only appears
// on the second ask, and a lookup that stopped at the first would never find it.
#[tokio::test]
async fn the_zone_walk_falls_through_to_the_apex() {
    let mock = MockApi::start(vec![
        MockRoute::new("GET", "/zones", empty()).when_query_contains("name=registry."),
        MockRoute::new("GET", "/zones", zone("zone-apex")).when_query_contains(&format!("name={APEX}")),
    ])
    .await;

    let dns = CloudflareDns::new_with_base("token", &mock.base);

    assert_eq!(dns.zone_for(HOSTNAME).await.expect("finds a zone"), "zone-apex");
    assert_eq!(
        mock.requests().len(),
        2,
        "the full hostname is asked first, then the apex"
    );
}

// A token that does not reach the zone is named as such. Falling through to a generic
// HTTP error would surface at issuance as a permissions failure that says nothing
// about which zone was missing.
#[tokio::test]
async fn a_hostname_in_no_reachable_zone_is_refused_by_name() {
    let mock = MockApi::start(vec![MockRoute::new("GET", "/zones", empty())]).await;

    let error = CloudflareDns::new_with_base("token", &mock.base)
        .zone_for(HOSTNAME)
        .await
        .expect_err("no zone is reachable");

    assert!(error.to_string().contains(HOSTNAME));
}

// The record has to land under `_acme-challenge` as a TXT, or the certificate
// authority looks where nothing was written and the order fails validation — which
// costs an issuance from the budget.
#[tokio::test]
async fn publishing_writes_a_txt_under_the_challenge_label() {
    let mock = MockApi::start(vec![
        MockRoute::new("GET", "/zones", zone("zone-apex")),
        MockRoute::new("POST", "/zones/zone-apex/dns_records", json!({ "success": true })),
    ])
    .await;

    CloudflareDns::new_with_base("token", &mock.base)
        .publish_txt(HOSTNAME, "challenge-value")
        .await
        .expect("publishes");

    let post = mock
        .requests()
        .into_iter()
        .find(|r| r.method == "POST")
        .expect("a record was created");

    assert!(post.body.contains("_acme-challenge.registry.bedrockvoicechat.com"));
    assert!(post.body.contains("\"type\":\"TXT\""));
    assert!(post.body.contains("challenge-value"));
}

// Cloudflare answering with `success: false` is a failure even though the HTTP status
// is 200. Reading only the status would leave the order to fail later at validation,
// pointing at DNS rather than at the write that never happened.
#[tokio::test]
async fn a_rejected_write_is_an_error_despite_the_status() {
    let mock = MockApi::start(vec![
        MockRoute::new("GET", "/zones", zone("zone-apex")),
        MockRoute::new(
            "POST",
            "/zones/zone-apex/dns_records",
            json!({ "success": false, "errors": [{ "message": "insufficient permissions" }] }),
        ),
    ])
    .await;

    let error = CloudflareDns::new_with_base("token", &mock.base)
        .publish_txt(HOSTNAME, "challenge-value")
        .await
        .expect_err("a rejected write is an error");

    assert!(error.to_string().contains("record create failed"));
}

// A retried order leaves more than one challenge record. Deleting only the first would
// leave the zone carrying a stale authorization, and the next order can then validate
// against a value the current one never issued.
#[tokio::test]
async fn cleanup_deletes_every_record_for_the_name() {
    let mock = MockApi::start(vec![
        MockRoute::new("GET", "/zones", zone("zone-apex")),
        MockRoute::new(
            "GET",
            "/zones/zone-apex/dns_records",
            json!({ "success": true, "result": [{ "id": "a" }, { "id": "b" }, { "id": "c" }] }),
        ),
        MockRoute::new("DELETE", "/zones/zone-apex/dns_records/a", json!({ "success": true })),
        MockRoute::new("DELETE", "/zones/zone-apex/dns_records/b", json!({ "success": true })),
        MockRoute::new("DELETE", "/zones/zone-apex/dns_records/c", json!({ "success": true })),
    ])
    .await;

    CloudflareDns::new_with_base("token", &mock.base)
        .cleanup_txt(HOSTNAME)
        .await
        .expect("cleans up");

    let deleted: Vec<String> = mock
        .requests()
        .into_iter()
        .filter(|r| r.method == "DELETE")
        .map(|r| r.path)
        .collect();

    assert_eq!(
        deleted,
        vec![
            "/zones/zone-apex/dns_records/a".to_string(),
            "/zones/zone-apex/dns_records/b".to_string(),
            "/zones/zone-apex/dns_records/c".to_string(),
        ]
    );
}
