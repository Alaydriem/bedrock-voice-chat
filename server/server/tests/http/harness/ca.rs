//! Generates a self-signed CA in a TempDir, mirroring the production `runtime::generate_ca`.

use anyhow::Result;
use rcgen::{
    CertificateParams, DistinguishedName, ExtendedKeyUsagePurpose, IsCa, KeyPair, KeyUsagePurpose,
};
use rocket::time::{Duration, OffsetDateTime};

pub struct GeneratedCa {
    pub cert_pem: String,
    pub key_pem: String,
}

impl GeneratedCa {
    pub fn generate(san: &[String]) -> Result<Self> {
        let kp = KeyPair::generate()?;

        let mut dn = DistinguishedName::new();
        dn.push(rcgen::DnType::CommonName, "Bedrock Voice Chat");

        let mut params = CertificateParams::new(san.to_vec())?;
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

        let cert = params.self_signed(&kp)?;
        Ok(Self {
            cert_pem: cert.pem(),
            key_pem: kp.serialize_pem(),
        })
    }
}
