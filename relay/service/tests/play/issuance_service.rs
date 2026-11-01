use std::net::IpAddr;
use std::sync::Arc;

use bvc_relay_service::db::Db;
use bvc_relay_service::play::{
    LocalSigner, PlayError, PlayIssuanceService, PlayNameService, PlaySigner,
};
use sea_orm::{ConnectionTrait, DatabaseConnection};
use x509_parser::extensions::GeneralName;
use x509_parser::pem::parse_x509_pem;

use crate::harness::{CsrFixture, PlayFixture};

struct Setup {
    conn: Arc<DatabaseConnection>,
    names: Arc<PlayNameService>,
    issuance: PlayIssuanceService,
}

async fn setup() -> Setup {
    setup_with(PlayFixture::config()).await
}

async fn setup_with(config: bvc_relay_service::config::PlayConfig) -> Setup {
    let conn = Arc::new(Db::connect("sqlite::memory:").await.expect("connects"));
    let names = PlayNameService::new_shared(conn.clone(), config);
    let issuance =
        PlayIssuanceService::new(names.clone(), PlaySigner::Local(LocalSigner::new()));

    Setup {
        conn,
        names,
        issuance,
    }
}

async fn claim(setup: &Setup, node: &str) -> String {
    let label = setup
        .names
        .claim(node, "203.0.113.7".parse::<IpAddr>().expect("an address"))
        .await
        .expect("claims");
    setup.names.fqdn(&label)
}

#[tokio::test]
async fn a_node_with_a_play_name_gets_a_leaf_whose_only_san_is_that_name() {
    let setup = setup().await;
    let fqdn = claim(&setup, "node-a").await;

    let chain = setup
        .issuance
        .issue("node-a", &CsrFixture::for_names(&[&fqdn]))
        .await
        .expect("issues");

    let (_, pem) = parse_x509_pem(chain.as_bytes()).expect("the chain starts with a certificate");
    let leaf = pem.parse_x509().expect("the leaf parses");
    let san = leaf
        .subject_alternative_name()
        .expect("the extension parses")
        .expect("the leaf has a SAN");
    let names: Vec<String> = san
        .value
        .general_names
        .iter()
        .map(|name| match name {
            GeneralName::DNSName(dns) => format!("DNS:{dns}"),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(names, vec![format!("DNS:{fqdn}")]);

    let lifetime = leaf.validity().not_after.timestamp() - leaf.validity().not_before.timestamp();
    assert_eq!(lifetime, 360 * 3600);
    assert_eq!(chain.matches("BEGIN CERTIFICATE").count(), 2);
}

#[tokio::test]
async fn a_csr_naming_another_play_name_is_refused() {
    let setup = setup().await;
    claim(&setup, "node-a").await;
    let other = claim(&setup, "node-b").await;

    let result = setup
        .issuance
        .issue("node-a", &CsrFixture::for_names(&[&other]))
        .await;

    assert!(matches!(result, Err(PlayError::CsrRejected(_))));
}

#[tokio::test]
async fn a_csr_with_an_extra_san_is_refused() {
    let setup = setup().await;
    let fqdn = claim(&setup, "node-a").await;

    let result = setup
        .issuance
        .issue("node-a", &CsrFixture::for_names(&[&fqdn, "www.play.bedrockvc.stream"]))
        .await;

    assert!(matches!(result, Err(PlayError::CsrRejected(_))));
}

#[tokio::test]
async fn a_csr_with_an_ip_san_is_refused() {
    let setup = setup().await;
    let fqdn = claim(&setup, "node-a").await;

    let result = setup
        .issuance
        .issue(
            "node-a",
            &CsrFixture::with_ip(&fqdn, "203.0.113.7".parse().expect("an address")),
        )
        .await;

    assert!(matches!(result, Err(PlayError::CsrRejected(_))));
}

#[tokio::test]
async fn a_csr_with_a_bad_signature_is_refused() {
    let setup = setup().await;
    let fqdn = claim(&setup, "node-a").await;

    let result = setup
        .issuance
        .issue("node-a", &CsrFixture::with_a_bad_signature(&[&fqdn]))
        .await;

    assert!(matches!(result, Err(PlayError::CsrRejected(_))));
}

// step-ca compares the subject with the token, so a request whose CN differs from the
// name would fail there with an error that names nothing useful.
#[tokio::test]
async fn a_csr_with_a_different_common_name_is_refused() {
    let setup = setup().await;
    let fqdn = claim(&setup, "node-a").await;

    let result = setup
        .issuance
        .issue("node-a", &CsrFixture::with_common_name(&fqdn, "someone-else.example"))
        .await;

    assert!(matches!(result, Err(PlayError::CsrRejected(_))));
}

#[tokio::test]
async fn a_csr_whose_common_name_is_the_play_name_is_accepted() {
    let setup = setup().await;
    let fqdn = claim(&setup, "node-a").await;

    let chain = setup
        .issuance
        .issue("node-a", &CsrFixture::with_common_name(&fqdn, &fqdn))
        .await
        .expect("issues");

    assert!(chain.contains("BEGIN CERTIFICATE"));
}

// The CA is shared infrastructure: a node looping on certificate requests would fill its
// database and use up its signing time.
#[tokio::test]
async fn a_node_beyond_its_daily_certificate_allowance_is_refused() {
    let mut config = PlayFixture::config();
    config.issuances_per_node_per_day = 2;
    let setup = setup_with(config).await;
    let fqdn = claim(&setup, "node-a").await;
    let request = CsrFixture::for_names(&[&fqdn]);

    setup.issuance.issue("node-a", &request).await.expect("first");
    setup.issuance.issue("node-a", &request).await.expect("second");

    assert!(matches!(
        setup.issuance.issue("node-a", &request).await,
        Err(PlayError::RateLimited)
    ));
}

#[tokio::test]
async fn a_node_without_a_play_name_is_refused() {
    let setup = setup().await;

    let result = setup
        .issuance
        .issue(
            "node-a",
            &CsrFixture::for_names(&["brave-otter-mesa.play.bedrockvc.stream"]),
        )
        .await;

    assert!(matches!(result, Err(PlayError::NoPlayName)));
}

#[tokio::test]
async fn a_released_name_is_refused() {
    let setup = setup().await;
    let fqdn = claim(&setup, "node-a").await;
    setup
        .conn
        .execute_unprepared("UPDATE play_name SET checked_in_at = 1 WHERE node_id = 'node-a'")
        .await
        .expect("ages the check-in");
    setup.names.release_stale().await.expect("sweeps");

    let result = setup
        .issuance
        .issue("node-a", &CsrFixture::for_names(&[&fqdn]))
        .await;

    assert!(matches!(result, Err(PlayError::NoPlayName)));
}
