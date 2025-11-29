use std::fs;

use anyhow::anyhow;
use rcgen::{
    Certificate, CertificateParams, DistinguishedName, ExtendedKeyUsagePurpose, KeyPair, SanType,
};
use time::{Duration, OffsetDateTime};

/// Creates a signs a certificate with a given CA.
pub fn sign_cert_with_ca(
    ca_cert: &Certificate,
    ca_key: &KeyPair,
    dn_name: &str,
) -> Result<(Certificate, KeyPair), anyhow::Error> {
    let mut dn = DistinguishedName::new();
    dn.push(rcgen::DnType::CommonName, dn_name);

    let mut params = CertificateParams::default();

    params.distinguished_name = dn;
    params.extended_key_usages = vec![
        ExtendedKeyUsagePurpose::ClientAuth,
        ExtendedKeyUsagePurpose::ServerAuth,
    ];
    params.not_before = OffsetDateTime::now_utc()
        .checked_sub(Duration::days(3))
        .unwrap();
    // @todo this should not be 9999 days...
    params.not_after = OffsetDateTime::now_utc() + Duration::days(9999);

    params.subject_alt_names = vec![
        SanType::DnsName(dn_name.try_into()?),
        SanType::DnsName(String::from("localhost").try_into()?),
        SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))),
        SanType::IpAddress(std::net::IpAddr::V6(std::net::Ipv6Addr::new(
            0, 0, 0, 0, 0, 0, 0, 1,
        ))),
    ];

    let key_pair = KeyPair::generate()?;
    let cert = params.signed_by(&key_pair, ca_cert, ca_key);
    match cert {
        Ok(cert) => Ok((cert, key_pair)),
        Err(_) => Err(anyhow!("Unable to generate certificate")),
    }
}

/// Retrieves the root certificate
pub fn get_root_ca(certificate_path: String) -> Result<(Certificate, KeyPair), anyhow::Error> {
    let root_ca_path_str = format!("{}/{}", &certificate_path, "ca.crt");
    let root_ca_key_path_str = format!("{}/{}", &certificate_path, "ca.key");
    let root_kp = KeyPair::from_pem(&fs::read_to_string(root_ca_key_path_str)?)?;
    let root_cp =
        CertificateParams::from_ca_cert_pem(&fs::read_to_string(root_ca_path_str).unwrap())
            .unwrap()
            .self_signed(&root_kp)
            .unwrap();

    Ok((root_cp, root_kp))
}
