use std::sync::Arc;

use anyhow::Result;
use bvc_server_lib::services::CertificateService;
use rcgen::{
    CertificateParams, DistinguishedName, ExtendedKeyUsagePurpose, IsCa, KeyPair, KeyUsagePurpose,
};
use rocket::time::{Duration, OffsetDateTime};
use tempfile::TempDir;

/// A self-signed CA in a temp directory plus a `CertificateService` that signs from it.
///
/// `CertificateService::new` loads the root CA off disk and fails if it is absent, so a test
/// that needs to mint a realistic leaf has to stand one up first. Mirrors production
/// `ServerRuntime::generate_ca`.
pub struct CertificateFixture {
    pub service: Arc<CertificateService>,
    _dir: TempDir,
}

impl CertificateFixture {
    pub fn create() -> Result<Self> {
        let dir = tempfile::tempdir()?;
        let certs_path = dir.path().join("certs");
        std::fs::create_dir_all(&certs_path)?;

        let key_pair = KeyPair::generate()?;
        let mut dn = DistinguishedName::new();
        dn.push(rcgen::DnType::CommonName, "Bedrock Voice Chat");

        let mut params =
            CertificateParams::new(vec!["localhost".to_string(), "127.0.0.1".to_string()])?;
        params.is_ca = IsCa::NoCa;
        params.distinguished_name = dn;
        params.use_authority_key_identifier_extension = true;
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign];
        params.extended_key_usages = vec![
            ExtendedKeyUsagePurpose::ClientAuth,
            ExtendedKeyUsagePurpose::ServerAuth,
        ];
        params.not_before = OffsetDateTime::now_utc()
            .checked_sub(Duration::days(3))
            .unwrap();
        params.not_after = OffsetDateTime::now_utc() + Duration::days(30);

        let certificate = params.self_signed(&key_pair)?;
        std::fs::write(certs_path.join("ca.crt"), certificate.pem())?;
        std::fs::write(certs_path.join("ca.key"), key_pair.serialize_pem())?;

        let service = CertificateService::new_shared(certs_path.to_str().unwrap())?;
        Ok(Self {
            service,
            _dir: dir,
        })
    }
}
