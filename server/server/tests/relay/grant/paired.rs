use std::collections::HashMap;

use bvc_relay::node::PeerTicket;
use bvc_relay::peer::PeerAuthority;
use bvc_server_lib::config::PeerConfig;
use bvc_server_lib::relay::GrantTable;
use bvc_server_lib::services::PairingService;
use iroh::{EndpointAddr, PublicKey};

use crate::harness::DatabaseFixture;

const NODE_HEX: &str = "aaaabbbbccccddddeeeeffff0000111122223333444455556666777788889999";

fn node_key() -> PublicKey {
    let bytes: [u8; 32] = hex::decode(NODE_HEX)
        .expect("hex")
        .try_into()
        .expect("32 bytes");

    PublicKey::from_bytes(&bytes).expect("valid key")
}

#[tokio::test]
async fn a_paired_row_authorizes_after_a_restart() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let code = PairingService::mint(&db.connection, "svc-bridge", PairingService::DEFAULT_TTL)
        .await
        .expect("mint");
    PairingService::redeem(&db.connection, NODE_HEX, &code, &["W1".to_string()])
        .await
        .expect("redeem");

    let table = GrantTable::from_config_and_db(&HashMap::new(), &db.connection)
        .await
        .expect("table");

    let scope = table
        .authorize(&node_key(), &["W1".to_string()])
        .expect("authorized");

    assert_eq!(scope.worlds, vec!["W1".to_string()]);
}

// Config is what an operator wrote deliberately. A paired row is what a bridge obtained,
// and it must not silently replace a declaration.
#[tokio::test]
async fn a_config_block_outranks_a_paired_row_for_the_same_node() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let code = PairingService::mint(&db.connection, "svc-bridge", PairingService::DEFAULT_TTL)
        .await
        .expect("mint");
    PairingService::redeem(
        &db.connection,
        NODE_HEX,
        &code,
        &["W1".to_string(), "W2".to_string()],
    )
    .await
    .expect("redeem");

    let peerlink = PeerTicket::mint(&EndpointAddr::new(node_key())).expect("mint peerlink");

    let mut peers = HashMap::new();
    peers.insert(
        "declared".to_string(),
        PeerConfig {
            peerlink,
            worlds: vec!["W1".to_string()],
            capabilities: PeerConfig::default_capabilities(),
        },
    );

    let table = GrantTable::from_config_and_db(&peers, &db.connection)
        .await
        .expect("table");

    let scope = table
        .authorize(&node_key(), &["W1".to_string(), "W2".to_string()])
        .expect("authorized");

    assert_eq!(
        scope.worlds,
        vec!["W1".to_string()],
        "the config block's filter must win"
    );
}

#[tokio::test]
async fn an_unpaired_undeclared_node_is_refused() {
    let db = DatabaseFixture::create().await.expect("fixture");

    let table = GrantTable::from_config_and_db(&HashMap::new(), &db.connection)
        .await
        .expect("table");

    assert!(table.authorize(&node_key(), &["W1".to_string()]).is_none());
}

// Revocation has to reach the running table as well as the row, or a revoked bridge keeps
// its link until the process restarts.
#[tokio::test]
async fn forgetting_a_label_drops_its_authorization() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let code = PairingService::mint(&db.connection, "svc-bridge", PairingService::DEFAULT_TTL)
        .await
        .expect("mint");
    PairingService::redeem(&db.connection, NODE_HEX, &code, &["W1".to_string()])
        .await
        .expect("redeem");

    let table = GrantTable::from_config_and_db(&HashMap::new(), &db.connection)
        .await
        .expect("table");

    let dropped = table.forget("svc-bridge");

    assert_eq!(dropped, 1);
    assert!(table.authorize(&node_key(), &["W1".to_string()]).is_none());
}
