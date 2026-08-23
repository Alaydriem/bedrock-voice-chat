use bvc_server_lib::services::CertificateRevocationService;

use crate::harness::DatabaseFixture;

// Comfortably beyond any certificate this suite mints; the pruner is the only reader.
const LATER: i64 = 4_102_444_800;

#[tokio::test]
async fn an_unrevoked_fingerprint_is_not_revoked() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let service = CertificateRevocationService::new_shared();

    assert!(!service.is_revoked(&db.connection, &"aa".repeat(32)).await);
}

#[tokio::test]
async fn a_revoked_fingerprint_is_revoked() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let service = CertificateRevocationService::new_shared();
    let fingerprint = "bb".repeat(32);

    service
        .revoke(&db.connection, &fingerprint, None, "test", LATER)
        .await
        .expect("revoke");

    assert!(service.is_revoked(&db.connection, &fingerprint).await);
}

// The negative answer is cached, so a revocation written after a miss has to evict it.
// Without eviction a ban takes up to the cache TTL to bite, which on a griefing report is
// the wrong answer.
#[tokio::test]
async fn revoking_evicts_a_cached_negative_answer() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let service = CertificateRevocationService::new_shared();
    let fingerprint = "cc".repeat(32);

    assert!(!service.is_revoked(&db.connection, &fingerprint).await);

    service
        .revoke(&db.connection, &fingerprint, None, "test", LATER)
        .await
        .expect("revoke");

    assert!(service.is_revoked(&db.connection, &fingerprint).await);
}

// Revoking the same certificate twice is what a double ban does. It must not error.
#[tokio::test]
async fn revoking_twice_is_idempotent() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let service = CertificateRevocationService::new_shared();
    let fingerprint = "dd".repeat(32);

    service
        .revoke(&db.connection, &fingerprint, None, "first", LATER)
        .await
        .expect("first revoke");
    service
        .revoke(&db.connection, &fingerprint, None, "second", LATER)
        .await
        .expect("second revoke");

    assert!(service.is_revoked(&db.connection, &fingerprint).await);
}

// An empty presented fingerprint must never match. A stored certificate that failed to parse
// yields no fingerprint, and a lookup that treated empty as a value could match the wrong row.
#[tokio::test]
async fn an_empty_fingerprint_is_never_revoked() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let service = CertificateRevocationService::new_shared();

    service
        .revoke(&db.connection, "", None, "test", LATER)
        .await
        .expect_err("an empty fingerprint cannot be revoked");

    assert!(!service.is_revoked(&db.connection, "").await);
}

// Revoking a stored PEM derives both the fingerprint and the expiry from the certificate, so
// a caller never has to compute either and cannot compute them differently.
#[tokio::test]
async fn revoking_a_pem_revokes_that_certificates_fingerprint() {
    use common::structs::certificate::CertificateFingerprint;

    let db = DatabaseFixture::create().await.expect("fixture");
    let service = CertificateRevocationService::new_shared();

    let key_pair = rcgen::KeyPair::generate().expect("keypair");
    let certificate = rcgen::CertificateParams::default()
        .self_signed(&key_pair)
        .expect("self-signed cert");
    let pem = certificate.pem();

    service
        .revoke_pem(&db.connection, &pem, None, "test")
        .await
        .expect("revoke_pem");

    let fingerprint = CertificateFingerprint::from_pem(&pem).expect("fingerprint");
    assert!(service.is_revoked(&db.connection, &fingerprint).await);
}

// A stored certificate that does not parse has no fingerprint, so there is nothing to
// revoke. Reporting success would leave an operator believing a ban had taken effect.
#[tokio::test]
async fn revoking_an_unparseable_pem_is_an_error() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let service = CertificateRevocationService::new_shared();

    service
        .revoke_pem(&db.connection, "not a certificate", None, "test")
        .await
        .expect_err("an unparseable certificate cannot be revoked");
}
