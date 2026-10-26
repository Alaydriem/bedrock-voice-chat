use std::sync::Arc;

use bvc_relay_service::db::Db;
use bvc_relay_service::dns::{CloudflareApi, RecordingApi, ZoneWriter};
use bvc_relay_service::validation::{AddressProbe, AddressVerifier};

async fn verifier(probe: AddressProbe) -> AddressVerifier {
    let conn = Arc::new(Db::connect("sqlite::memory:").await.expect("connects"));
    let zone = Arc::new(ZoneWriter::new(
        conn,
        CloudflareApi::Recording(Arc::new(RecordingApi::new())),
        "bedrockvc.stream".to_string(),
    ));
    AddressVerifier::new(probe, zone)
}

#[tokio::test]
async fn the_verifier_probes_the_fully_qualified_name_on_the_given_port() {
    let verifier = verifier(AddressProbe::ForPort(
        "creeper-diorite-badlands.bedrockvc.stream".to_string(),
        8443,
        "nonce".to_string(),
    ))
    .await;

    assert!(
        verifier
            .serves_nonce("creeper-diorite-badlands", "203.0.113.10", 8443, "nonce")
            .await
    );
    assert!(
        !verifier
            .serves_nonce("creeper-diorite-badlands", "203.0.113.10", 443, "nonce")
            .await
    );
}
