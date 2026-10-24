use std::sync::Arc;

use bvc_relay_service::db::Db;
use bvc_relay_service::dns::{CloudflareApi, RecordingApi, ZoneWriter};

async fn writer() -> (ZoneWriter, Arc<RecordingApi>) {
    let conn = Arc::new(Db::connect("sqlite::memory:").await.expect("connects"));
    let recording = Arc::new(RecordingApi::new());
    let writer = ZoneWriter::new(
        conn,
        CloudflareApi::Recording(recording.clone()),
        "bedrockvc.stream".to_string(),
    );
    (writer, recording)
}

// Cleanup deletes by the id the ledger stored, not by matching content. Matching on
// content cannot tell two identical challenge values apart, and leaves a record
// behind whenever a retry wrote the same string twice.
#[tokio::test]
async fn cleanup_deletes_every_record_the_ledger_holds_for_a_name() {
    let (writer, api) = writer().await;
    writer
        .publish_txt("creeper-diorite-badlands", "first-value")
        .await
        .expect("publishes");
    writer
        .publish_txt("creeper-diorite-badlands", "second-value")
        .await
        .expect("publishes a second value");

    writer
        .cleanup_txt("creeper-diorite-badlands")
        .await
        .expect("cleans up");

    assert!(api.live_ids().is_empty(), "both records must be deleted");
}

// Two concurrent challenge values coexist. A single-value store would clobber its
// own challenge on an overlapping retry.
#[tokio::test]
async fn two_challenge_values_for_one_name_coexist() {
    let (writer, api) = writer().await;

    writer
        .publish_txt("creeper-diorite-badlands", "first-value")
        .await
        .expect("publishes");
    writer
        .publish_txt("creeper-diorite-badlands", "second-value")
        .await
        .expect("publishes a second value");

    assert_eq!(api.live_ids().len(), 2);
}

// The challenge record is written at `_acme-challenge.<name>.<zone>`, which is where
// the certificate authority looks.
#[tokio::test]
async fn a_challenge_is_written_beneath_the_acme_challenge_label() {
    let (writer, api) = writer().await;

    writer
        .publish_txt("creeper-diorite-badlands", "value")
        .await
        .expect("publishes");

    assert_eq!(
        api.created_names(),
        vec!["_acme-challenge.creeper-diorite-badlands.bedrockvc.stream".to_string()]
    );
}

// A second address replaces the first. Two A records would round-robin between an
// operator's current host and one they moved off.
#[tokio::test]
async fn a_second_address_replaces_the_first() {
    let (writer, api) = writer().await;
    writer
        .publish_address(
            "creeper-diorite-badlands",
            "203.0.113.10".parse().expect("an address"),
        )
        .await
        .expect("publishes an address");

    writer
        .publish_address(
            "creeper-diorite-badlands",
            "203.0.113.20".parse().expect("an address"),
        )
        .await
        .expect("republishes");

    assert_eq!(api.live_ids().len(), 1);
}

// Withdrawing a name removes its address record as well as any challenge left
// behind, so a suspended registration stops resolving entirely.
#[tokio::test]
async fn withdrawing_a_name_removes_every_record_for_it() {
    let (writer, api) = writer().await;
    writer
        .publish_address(
            "creeper-diorite-badlands",
            "203.0.113.10".parse().expect("an address"),
        )
        .await
        .expect("publishes an address");
    writer
        .publish_txt("creeper-diorite-badlands", "value")
        .await
        .expect("publishes a challenge");

    writer
        .withdraw("creeper-diorite-badlands")
        .await
        .expect("withdraws");

    assert!(api.live_ids().is_empty());
}

// Withdrawing one name leaves another operator's records alone. The filter is on the
// fully qualified name, so a prefix that happens to overlap must not match.
#[tokio::test]
async fn withdrawing_one_name_leaves_another_untouched() {
    let (writer, api) = writer().await;
    writer
        .publish_address(
            "creeper-diorite-badlands",
            "203.0.113.10".parse().expect("an address"),
        )
        .await
        .expect("publishes");
    writer
        .publish_address(
            "redstone-piglin-taiga",
            "203.0.113.20".parse().expect("an address"),
        )
        .await
        .expect("publishes");

    writer
        .withdraw("creeper-diorite-badlands")
        .await
        .expect("withdraws");

    assert_eq!(api.live_ids().len(), 1);
}

// The wire boundary in both directions. A name leaves qualified and comes back
// qualified, and the label is what everything inside the registry is keyed by — the
// registration, the ledger, the retired list and the issuance budget.
#[tokio::test]
async fn a_qualified_name_round_trips_to_its_label() {
    let (writer, _) = writer().await;

    let fqdn = writer.address_fqdn("tidy-allay-lagoon");
    assert_eq!(fqdn, "tidy-allay-lagoon.bedrockvc.stream");
    assert_eq!(
        writer.label_of(&fqdn),
        Some("tidy-allay-lagoon".to_string())
    );
}

// A name outside this zone has no label here. Accepting one would let a node publish a
// challenge for a name the registry does not control, which is the whole point of the
// ownership check that follows it.
#[tokio::test]
async fn a_name_outside_the_zone_has_no_label() {
    let (writer, _) = writer().await;

    assert_eq!(writer.label_of("tidy-allay-lagoon.example.com"), None);
    assert_eq!(writer.label_of("bedrockvc.stream"), None);
    assert_eq!(writer.label_of(".bedrockvc.stream"), None);
    // A deeper name is not a label: `evil.tidy-allay-lagoon.bedrockvc.stream` would
    // otherwise strip to something that is not the registration it claims to be.
    assert_eq!(
        writer.label_of("evil.tidy-allay-lagoon.bedrockvc.stream"),
        None
    );
}

// An IPv6 address is published as AAAA. The zone refuses an IPv6 address in an A
// record, so an operator with only an IPv6 host would otherwise get no record at all.
#[tokio::test]
async fn an_ipv6_address_is_published_as_aaaa() {
    let (writer, api) = writer().await;
    writer
        .publish_address(
            "creeper-diorite-badlands",
            "2001:db8::10".parse().expect("an address"),
        )
        .await
        .expect("publishes");

    assert_eq!(
        api.created_records(),
        vec![(
            "AAAA".to_string(),
            "creeper-diorite-badlands.bedrockvc.stream".to_string(),
            "2001:db8::10".to_string()
        )]
    );
}

// Withdrawing the address leaves a challenge in flight alone. A new declaration
// withdraws the old address, and must not break an issuance running alongside it.
#[tokio::test]
async fn withdrawing_the_address_leaves_challenges_alone() {
    let (writer, api) = writer().await;
    writer
        .publish_address(
            "creeper-diorite-badlands",
            "203.0.113.10".parse().expect("an address"),
        )
        .await
        .expect("publishes an address");
    writer
        .publish_txt("creeper-diorite-badlands", "value")
        .await
        .expect("publishes a challenge");

    writer
        .withdraw_address("creeper-diorite-badlands")
        .await
        .expect("withdraws");

    assert_eq!(api.live_ids(), vec!["record-1".to_string()]);
    assert_eq!(
        writer
            .open_challenges("creeper-diorite-badlands")
            .await
            .expect("counts"),
        1
    );
}

// Pruning removes challenge records older than the cutoff, across every name, and
// never touches an address record. Nothing on the wire says when a challenge is done,
// so without this they would accumulate until the name was withdrawn.
#[tokio::test]
async fn pruning_removes_old_challenges_and_keeps_addresses() {
    let (writer, api) = writer().await;
    writer
        .publish_address(
            "creeper-diorite-badlands",
            "203.0.113.10".parse().expect("an address"),
        )
        .await
        .expect("publishes an address");
    writer
        .publish_txt("creeper-diorite-badlands", "one")
        .await
        .expect("publishes");
    writer
        .publish_txt("redstone-piglin-taiga", "two")
        .await
        .expect("publishes");

    // A cutoff in the past keeps everything; one in the future clears every challenge.
    writer.prune_challenges_before(0).await.expect("prunes");
    assert_eq!(api.live_ids().len(), 3);

    writer
        .prune_challenges_before(i64::MAX)
        .await
        .expect("prunes");
    assert_eq!(api.live_ids(), vec!["record-0".to_string()]);
}
