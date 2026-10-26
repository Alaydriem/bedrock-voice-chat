use std::sync::Arc;

use bvc_server_lib::config::{Acme, AcmeProviderKind};
use bvc_server_lib::services::acme::{AcmeService, AcmeStorage};
use rcgen::{CertificateParams, KeyPair};
use tempfile::TempDir;
use time::OffsetDateTime;

use crate::harness::DatabaseFixture;

const CLOSED_DIRECTORY: &str = "http://127.0.0.1:1/directory";

fn expired_cert() -> String {
    let mut params = CertificateParams::new(vec!["test.example.com".to_string()]).unwrap();
    params.not_before = OffsetDateTime::now_utc() - time::Duration::days(10);
    params.not_after = OffsetDateTime::now_utc() - time::Duration::days(1);
    let key = KeyPair::generate().unwrap();
    params.self_signed(&key).unwrap().pem()
}

#[tokio::test]
async fn a_stored_certificate_is_served_when_issuance_fails() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().unwrap();
    let certs_path = dir.path().to_str().unwrap();
    let names = vec!["test.example.com".to_string()];

    let mut acme = Acme::default();
    acme.email = "ops@example.com".to_string();
    acme.provider = Some(AcmeProviderKind::Cloudflare);
    acme.api_token = Some("token".to_string());
    acme.directory = CLOSED_DIRECTORY.to_string();

    let conn = Arc::new(db.connection.clone());
    let storage = AcmeStorage::new(
        certs_path,
        conn.clone(),
        CLOSED_DIRECTORY.to_string(),
        names.clone(),
    );
    storage.store_account_credentials("{}").await.unwrap();
    storage
        .store_certificate(&expired_cert(), "key-pem")
        .await
        .unwrap();

    let service = AcmeService::new(acme, &names, certs_path, conn).unwrap();
    let paths = service.ensure_certificate().await.unwrap();

    assert_eq!(
        std::path::PathBuf::from(&paths.certificate),
        storage.certificate_path()
    );
    assert!(std::path::Path::new(&paths.certificate).exists());
}
