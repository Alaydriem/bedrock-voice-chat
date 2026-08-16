//! Certificate service for player authentication

use std::fs;
use std::sync::Arc;

use anyhow::anyhow;
use rcgen::{
    Certificate, CertificateParams, DistinguishedName, ExtendedKeyUsagePurpose, Issuer, KeyPair,
    SanType,
};
use time::{Duration, OffsetDateTime};

/// Service for certificate operations for player authentication.
/// Caches the root CA issuer (cert + keypair) to avoid repeated file I/O.
pub struct CertificateService {
    issuer: Issuer<'static, KeyPair>,
}

impl CertificateService {
    pub fn new(certs_path: &str) -> Result<Self, anyhow::Error> {
        Ok(Self {
            issuer: Self::load_root_ca(certs_path)?,
        })
    }

    pub fn new_shared(certs_path: &str) -> Result<Arc<Self>, anyhow::Error> {
        Ok(Arc::new(Self::new(certs_path)?))
    }

    pub fn sign_player_cert(
        &self,
        player_name: &str,
        game: &common::Game,
    ) -> Result<(Certificate, KeyPair), anyhow::Error> {
        self.build_signed_cert(&format!("{}:{}", game.as_str(), player_name), player_name)
    }

    /// Single cert-issuance path shared by all leaf-cert callers. The CN is set
    /// to `cn_identity` and a DNS SAN is added for `san_host`; all other cert
    /// parameters (EKU, validity window, loopback SANs) are fixed here.
    fn build_signed_cert(
        &self,
        cn_identity: &str,
        san_host: &str,
    ) -> Result<(Certificate, KeyPair), anyhow::Error> {
        let mut dn = DistinguishedName::new();
        dn.push(rcgen::DnType::CommonName, cn_identity.to_string());

        let mut params = CertificateParams::default();

        params.distinguished_name = dn;
        params.extended_key_usages = vec![
            ExtendedKeyUsagePurpose::ClientAuth,
            ExtendedKeyUsagePurpose::ServerAuth,
        ];
        params.not_before = OffsetDateTime::now_utc()
            .checked_sub(Duration::days(3))
            .unwrap();
        params.not_after = OffsetDateTime::now_utc() + Duration::days(90);

        params.subject_alt_names = vec![
            SanType::DnsName(san_host.to_string().try_into()?),
            SanType::DnsName(String::from("localhost").try_into()?),
            SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))),
            SanType::IpAddress(std::net::IpAddr::V6(std::net::Ipv6Addr::new(
                0, 0, 0, 0, 0, 0, 0, 1,
            ))),
        ];

        let key_pair = KeyPair::generate()?;
        match params.signed_by(&key_pair, &self.issuer) {
            Ok(cert) => Ok((cert, key_pair)),
            Err(_) => Err(anyhow!("Unable to generate certificate")),
        }
    }

    fn load_root_ca(certificate_path: &str) -> Result<Issuer<'static, KeyPair>, anyhow::Error> {
        let root_ca_path_str = format!("{}/{}", certificate_path, "ca.crt");
        let root_ca_key_path_str = format!("{}/{}", certificate_path, "ca.key");
        let root_kp = KeyPair::from_pem(&fs::read_to_string(root_ca_key_path_str)?)?;
        let issuer = Issuer::from_ca_cert_pem(&fs::read_to_string(root_ca_path_str)?, root_kp)?;
        Ok(issuer)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::runtime::ca_cert::CaCertManager;

    fn make_service() -> (CertificateService, TempDir) {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().to_str().expect("utf-8 path");
        CaCertManager::new(path)
            .ensure(&[String::from("localhost")])
            .expect("CA generation should succeed");
        let service = CertificateService::new(path).expect("CertificateService::new");
        (service, dir)
    }

    fn extract_cn(cert: &Certificate) -> String {
        let pem = cert.pem();
        let (_, x509) = x509_parser::pem::parse_x509_pem(pem.as_bytes()).unwrap();
        let x509 = x509.parse_x509().unwrap();
        x509.subject()
            .iter_common_name()
            .next()
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned()
    }

}
