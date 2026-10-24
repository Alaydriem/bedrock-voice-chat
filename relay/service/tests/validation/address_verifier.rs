use std::sync::Arc;

use bvc_relay_service::db::Db;
use bvc_relay_service::dns::{CloudflareApi, RecordingApi, ZoneWriter};
use bvc_relay_service::enroll::EnrollSessions;
use bvc_relay_service::validation::{AddressProbe, AddressVerifier};

async fn verifier(probe: AddressProbe) -> AddressVerifier {
    let conn = Arc::new(Db::connect("sqlite::memory:").await.expect("connects"));
    let zone = Arc::new(ZoneWriter::new(
        conn,
        CloudflareApi::Recording(Arc::new(RecordingApi::new())),
        "bedrockvc.stream".to_string(),
    ));
    AddressVerifier::new(EnrollSessions::new_shared(), probe, zone)
}

// The probe is aimed at the fully qualified name. The stored label is not a hostname
// and never appears on a certificate, so a probe against it fails validation on every
// server — and three days of that suspends them all.
#[tokio::test]
async fn the_probe_is_aimed_at_the_fully_qualified_name() {
    let verifier = verifier(AddressProbe::For(
        "creeper-diorite-badlands.bedrockvc.stream".to_string(),
    ))
    .await;

    assert!(
        verifier
            .serves_nonce("creeper-diorite-badlands", "203.0.113.10", "nonce")
            .await
    );
}

// Without a held session there is nobody to answer the challenge, which is a failed
// identity check rather than an error.
#[tokio::test]
async fn a_node_with_no_session_fails_the_challenge() {
    let verifier = verifier(AddressProbe::Fixed(true)).await;

    let node = iroh::SecretKey::from_bytes(&[7u8; 32]).public().to_string();
    assert_eq!(verifier.challenge(&node).await, None);
}
