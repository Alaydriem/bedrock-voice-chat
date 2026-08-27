use std::sync::Arc;

use bvc_relay_service::acme::CertificateIssuer;
use bvc_relay_service::config::AcmeConfig;
use bvc_relay_service::db::Db;
use bvc_relay_service::storage::CertificateStore;
use time::Duration;

use crate::harness::CertificateFixture;

const HOSTNAME: &str = "registry.example.test";

async fn store() -> Arc<CertificateStore> {
    let conn = Arc::new(Db::connect("sqlite::memory:").await.expect("connects"));
    CertificateStore::new_shared(conn)
}

fn issuer(store: Arc<CertificateStore>) -> CertificateIssuer {
    CertificateIssuer::new(
        AcmeConfig {
            email: "ops@example.test".to_string(),
            api_token: "unused".to_string(),
            directory: "https://acme.invalid/directory".to_string(),
        },
        HOSTNAME.to_string(),
        store,
    )
}

// The first start against an empty database. Nothing stored means an issuance is owed,
// and a registry that decided otherwise would come up with no certificate to serve.
#[tokio::test]
async fn an_absent_certificate_needs_issuance() {
    let issuer = issuer(store().await);

    assert_eq!(issuer.current().await.expect("reads"), None);
}

// The ordinary steady state. Every issuance draws on a rate limit shared with every
// assigned name, so re-ordering a certificate that has months left spends budget an
// operator then cannot have.
#[tokio::test]
async fn a_certificate_outside_the_renewal_window_is_used_as_is() {
    let store = store().await;
    let fixture = CertificateFixture::issue(HOSTNAME, Duration::days(89));
    store.write(HOSTNAME, &fixture.material).await.expect("stores");

    assert_eq!(
        issuer(store).current().await.expect("reads"),
        Some(fixture.material)
    );
}

// The renewal itself. A certificate inside the window must be reordered while it is
// still valid; waiting for expiry means an outage rather than a renewal.
#[tokio::test]
async fn a_certificate_inside_the_renewal_window_needs_issuance() {
    let store = store().await;
    let fixture = CertificateFixture::issue(HOSTNAME, Duration::days(10));
    store.write(HOSTNAME, &fixture.material).await.expect("stores");

    assert_eq!(issuer(store).current().await.expect("reads"), None);
}

// The boundary is the constant, not a number repeated here.
#[tokio::test]
async fn the_renewal_boundary_is_the_declared_window() {
    let inside = store().await;
    inside
        .write(
            HOSTNAME,
            &CertificateFixture::issue(
                HOSTNAME,
                Duration::days(CertificateIssuer::RENEWAL_WINDOW_DAYS - 1),
            )
            .material,
        )
        .await
        .expect("stores");

    let outside = store().await;
    outside
        .write(
            HOSTNAME,
            &CertificateFixture::issue(
                HOSTNAME,
                Duration::days(CertificateIssuer::RENEWAL_WINDOW_DAYS + 1),
            )
            .material,
        )
        .await
        .expect("stores");

    assert!(issuer(inside).current().await.expect("reads").is_none());
    assert!(issuer(outside).current().await.expect("reads").is_some());
}

// Material this process itself wrote but cannot parse is replaced rather than trusted.
// Refusing to start on it would leave a registry that cannot recover without somebody
// editing the database by hand.
#[tokio::test]
async fn an_unparseable_certificate_needs_issuance() {
    let store = store().await;
    store
        .write(
            HOSTNAME,
            &bvc_relay_service::storage::CertificateMaterial::new(
                "not a certificate".to_string(),
                "not a key".to_string(),
            ),
        )
        .await
        .expect("stores");

    assert_eq!(issuer(store).current().await.expect("reads"), None);
}

// Keyed by hostname, so renaming the registry cannot serve the previous name's
// certificate. The lookup misses and an issuance is owed instead.
#[tokio::test]
async fn a_certificate_for_another_hostname_is_not_used() {
    let store = store().await;
    store
        .write(
            "someone.else.test",
            &CertificateFixture::issue("someone.else.test", Duration::days(89)).material,
        )
        .await
        .expect("stores");

    assert_eq!(issuer(store).current().await.expect("reads"), None);
}

// A renewal replaces the row rather than colliding with it. An insert would fail on the
// second issuance — sixty days after anyone last watched this happen.
#[tokio::test]
async fn a_renewal_replaces_the_stored_certificate() {
    let store = store().await;
    let first = CertificateFixture::issue(HOSTNAME, Duration::days(40));
    let second = CertificateFixture::issue(HOSTNAME, Duration::days(90));

    store.write(HOSTNAME, &first.material).await.expect("stores");
    store
        .write(HOSTNAME, &second.material)
        .await
        .expect("replaces");

    assert_eq!(
        store.read(HOSTNAME).await.expect("reads"),
        Some(second.material)
    );
}
