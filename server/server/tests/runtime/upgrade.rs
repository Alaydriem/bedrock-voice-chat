use std::sync::Arc;

use bvc_relay::node::NodeIdentity;
use bvc_server_lib::runtime::access_token::AccessTokenManager;
use bvc_server_lib::runtime::ca_cert::KeyMatch;
use bvc_server_lib::runtime::{CaStore, NodeKeyStore, SecretName, SecretStore};
use bvc_server_lib::services::acme::AcmeStorage;
use rcgen::KeyPair;
use x509_parser::prelude::*;

use crate::harness::{Beta20Fixture, DatabaseFixture};

const DIRECTORY: &str = "https://acme-v02.api.letsencrypt.org/directory";

fn acme_names() -> Vec<String> {
    vec!["localhost".to_string()]
}

/// What one boot resolves, in the order `ServerRuntime::start_async` resolves it.
struct Resolved {
    ca_cert: String,
    ca_key: String,
    token: String,
    node_secret: [u8; 32],
}

async fn reconcile(fixture: &Beta20Fixture) -> Resolved {
    let conn = Arc::new(fixture.connection.clone());

    let (ca_cert, ca_key) = CaStore::ensure(
        &fixture.connection,
        fixture.certs_path(),
        &Beta20Fixture::sans(),
    )
    .await
    .expect("ca");

    let token = AccessTokenManager::new(fixture.certs_path())
        .resolve(&fixture.connection, "")
        .await
        .expect("token");

    let node_secret = NodeKeyStore::new(fixture.certs_path())
        .resolve(&fixture.connection)
        .await
        .expect("node key");

    AcmeStorage::new(
        fixture.certs_path(),
        conn,
        DIRECTORY.to_string(),
        acme_names(),
    )
    .import_legacy()
    .await
    .expect("acme import");

    Resolved {
        ca_cert,
        ca_key,
        token,
        node_secret,
    }
}

fn superseded_files(dir: &std::path::Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .expect("read_dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains(".superseded-"))
        .collect()
}

// The failure that started this work. Every player certificate ever issued chains to the
// authority the deployment already has; adopting a different one locks every player out.
#[tokio::test]
async fn the_existing_authority_is_adopted_unchanged() {
    let fixture = Beta20Fixture::create().await.expect("fixture");
    let before_cert = fixture.ca_cert_pem().to_string();
    let before_key = fixture.ca_key_pem().to_string();

    fixture.migrate().await.expect("migrate");
    let resolved = reconcile(&fixture).await;

    assert_eq!(
        resolved.ca_cert, before_cert,
        "the trust anchor must survive the upgrade byte for byte"
    );
    assert_eq!(resolved.ca_key, before_key);
}

// The assertion that actually matters to a user: their existing credential still works.
#[tokio::test]
async fn a_player_certificate_issued_before_the_upgrade_still_verifies() {
    let fixture = Beta20Fixture::create().await.expect("fixture");
    let player_pem = fixture.player_cert_pem().to_string();

    fixture.migrate().await.expect("migrate");
    let resolved = reconcile(&fixture).await;

    let (_, ca_pem) =
        x509_parser::pem::parse_x509_pem(resolved.ca_cert.as_bytes()).expect("ca pem");
    let (_, ca) = X509Certificate::from_der(&ca_pem.contents).expect("ca der");
    let (_, player_pem_parsed) =
        x509_parser::pem::parse_x509_pem(player_pem.as_bytes()).expect("player pem");
    let (_, player) =
        X509Certificate::from_der(&player_pem_parsed.contents).expect("player der");

    assert!(
        player.verify_signature(Some(ca.public_key())).is_ok(),
        "a certificate issued before the upgrade must still chain to the authority"
    );
}

#[tokio::test]
async fn the_access_token_survives_the_upgrade() {
    let fixture = Beta20Fixture::create().await.expect("fixture");

    fixture.migrate().await.expect("migrate");
    let resolved = reconcile(&fixture).await;

    assert_eq!(resolved.token, Beta20Fixture::ACCESS_TOKEN);
    assert_eq!(
        SecretStore::read(&fixture.connection, SecretName::MinecraftAccessToken)
            .await
            .expect("read"),
        Some(Beta20Fixture::ACCESS_TOKEN.to_string())
    );
}

#[tokio::test]
async fn the_node_identity_survives_the_upgrade() {
    let fixture = Beta20Fixture::create().await.expect("fixture");
    let before = fixture.node_id().to_string();

    fixture.migrate().await.expect("migrate");
    let resolved = reconcile(&fixture).await;

    assert_eq!(
        NodeIdentity::from_secret_bytes(&resolved.node_secret)
            .node_id()
            .to_string(),
        before,
        "every far-side peer block names this key"
    );
}

#[tokio::test]
async fn the_acme_account_survives_the_upgrade() {
    let fixture = Beta20Fixture::create().await.expect("fixture");

    fixture.migrate().await.expect("migrate");
    reconcile(&fixture).await;

    let storage = AcmeStorage::new(
        fixture.certs_path(),
        Arc::new(fixture.connection.clone()),
        DIRECTORY.to_string(),
        acme_names(),
    );
    assert_eq!(
        storage.load_account_credentials().await.expect("load"),
        Some(Beta20Fixture::ACME_ACCOUNT.to_string()),
        "re-registering costs an ACME registration for nothing"
    );
}

// An upgrade adopts what is on disk, so nothing is displaced. A superseded file appearing
// during an upgrade is a defect, not a diagnostic.
#[tokio::test]
async fn the_upgrade_supersedes_nothing() {
    let fixture = Beta20Fixture::create().await.expect("fixture");

    fixture.migrate().await.expect("migrate");
    reconcile(&fixture).await;

    let superseded = superseded_files(fixture.certs_dir.path());
    assert!(
        superseded.is_empty(),
        "an upgrade displaces nothing, got {superseded:?}"
    );
}

// A restart must change nothing. A boot that keeps rewriting material is a boot that can
// keep getting it wrong.
#[tokio::test]
async fn a_second_boot_after_the_upgrade_is_a_no_op() {
    let fixture = Beta20Fixture::create().await.expect("fixture");

    fixture.migrate().await.expect("migrate");
    let first = reconcile(&fixture).await;
    let second = reconcile(&fixture).await;

    assert_eq!(first.ca_cert, second.ca_cert);
    assert_eq!(first.ca_key, second.ca_key);
    assert_eq!(first.token, second.token);
    assert_eq!(first.node_secret, second.node_secret);
    assert!(superseded_files(fixture.certs_dir.path()).is_empty());
}

// The CA pair reaching the database must always correspond, whatever put it there.
#[tokio::test]
async fn the_stored_authority_always_corresponds() {
    let fixture = Beta20Fixture::create().await.expect("fixture");
    fixture.migrate().await.expect("migrate");
    let resolved = reconcile(&fixture).await;

    assert!(
        KeyMatch::matches(
            &resolved.ca_cert,
            &KeyPair::from_pem(&resolved.ca_key).expect("kp")
        ),
        "a mismatched pair must never reach the database"
    );
}

// The other half of the contract: a container with nothing on disk writes only what a
// consumer opens by path.
#[tokio::test]
async fn a_fresh_install_writes_no_scalar_secret_to_disk() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().to_str().unwrap();

    CaStore::ensure(&db.connection, path, &Beta20Fixture::sans())
        .await
        .expect("ca");
    AccessTokenManager::new(path)
        .resolve(&db.connection, "")
        .await
        .expect("token");
    NodeKeyStore::new(path)
        .resolve(&db.connection)
        .await
        .expect("node");

    assert!(dir.path().join("ca.crt").exists());
    assert!(dir.path().join("ca.key").exists());
    assert!(
        !dir.path().join("access_token").exists(),
        "the access token file is what makes a container need a volume"
    );
    assert!(
        !dir.path().join("node.key").exists(),
        "the node key file is what makes a container need a volume"
    );
}

// A container replacement after the upgrade: the database is all that survives, and the
// deployment must come back with the same identity.
#[tokio::test]
async fn an_empty_container_after_the_upgrade_restores_every_secret() {
    let fixture = Beta20Fixture::create().await.expect("fixture");
    fixture.migrate().await.expect("migrate");
    let original = reconcile(&fixture).await;

    // A new container: the same database, an empty certs directory.
    let fresh = tempfile::tempdir().expect("tempdir");
    let path = fresh.path().to_str().unwrap();

    let (ca_cert, ca_key) = CaStore::ensure(&fixture.connection, path, &Beta20Fixture::sans())
        .await
        .expect("ca");
    let token = AccessTokenManager::new(path)
        .resolve(&fixture.connection, "")
        .await
        .expect("token");
    let node_secret = NodeKeyStore::new(path)
        .resolve(&fixture.connection)
        .await
        .expect("node key");

    assert_eq!(ca_cert, original.ca_cert, "the trust anchor is restored");
    assert_eq!(ca_key, original.ca_key);
    assert_eq!(token, original.token);
    assert_eq!(node_secret, original.node_secret);
    assert!(fresh.path().join("ca.crt").exists());
}

// Negative control for the test above. If the upgrade had adopted a different authority —
// the failure this work exists to prevent — the pre-upgrade certificate must stop verifying.
// Without this, a chain assertion that always passes would look like proof and be none.
#[tokio::test]
async fn a_player_certificate_does_not_verify_against_a_different_authority() {
    let fixture = Beta20Fixture::create().await.expect("fixture");
    let player_pem = fixture.player_cert_pem().to_string();

    // A deployment that minted a fresh authority instead of adopting the existing one.
    let other_dir = tempfile::tempdir().expect("tempdir");
    let other_db = DatabaseFixture::create().await.expect("fixture");
    let (other_ca, _other_key) = CaStore::ensure(
        &other_db.connection,
        other_dir.path().to_str().unwrap(),
        &Beta20Fixture::sans(),
    )
    .await
    .expect("other ca");

    let (_, ca_pem) = x509_parser::pem::parse_x509_pem(other_ca.as_bytes()).expect("ca pem");
    let (_, ca) = X509Certificate::from_der(&ca_pem.contents).expect("ca der");
    let (_, player_pem_parsed) =
        x509_parser::pem::parse_x509_pem(player_pem.as_bytes()).expect("player pem");
    let (_, player) = X509Certificate::from_der(&player_pem_parsed.contents).expect("player der");

    assert!(
        player.verify_signature(Some(ca.public_key())).is_err(),
        "a certificate from another authority must not verify, or the assertion above proves \
         nothing"
    );
}
