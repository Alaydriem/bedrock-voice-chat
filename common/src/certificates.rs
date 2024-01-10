use std::fs;

use rcgen::{
    KeyPair,
    Certificate,
    CertificateParams,
    DistinguishedName,
    PKCS_ED25519,
    SanType,
    ExtendedKeyUsagePurpose,
};
use rocket::time::OffsetDateTime;
use rocket::time::Duration;
use anyhow::anyhow;

/// Creates a signs a certificate with a given CA.
pub fn sign_cert_with_ca(
    ca_cert: &Certificate,
    dn_name: &str
) -> Result<(String, String), anyhow::Error> {
    let mut dn = DistinguishedName::new();
    dn.push(rcgen::DnType::CommonName, dn_name);

    let mut params = CertificateParams::default();

    params.distinguished_name = dn;
    params.alg = &PKCS_ED25519;
    params.extended_key_usages = vec![
        ExtendedKeyUsagePurpose::ClientAuth,
        ExtendedKeyUsagePurpose::ServerAuth
    ];
    params.not_before = OffsetDateTime::now_utc().checked_sub(Duration::days(3)).unwrap();
    params.not_after = OffsetDateTime::now_utc() + Duration::days(60);

    params.subject_alt_names = vec![
        SanType::DnsName(dn_name.to_string()),
        SanType::DnsName(String::from("localhost")),
        SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))),
        SanType::IpAddress(std::net::IpAddr::V6(std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)))
    ];

    let cert = Certificate::from_params(params)?;
    let cert_signed = cert.serialize_pem_with_signer(&ca_cert)?;

    Ok((cert_signed, cert.serialize_private_key_pem()))
}

/// Retrieves the root certificate
pub fn get_root_ca(certificate_path: String) -> Result<Certificate, anyhow::Error> {
    let root_ca_path_str = format!("{}/{}", &certificate_path, "ca.crt");
    let root_ca_key_path_str = format!("{}/{}", &certificate_path, "ca.key");
    let root_kp = KeyPair::from_pem(&fs::read_to_string(root_ca_key_path_str)?)?;
    let root_cp = CertificateParams::from_ca_cert_pem(
        &fs::read_to_string(root_ca_path_str).unwrap(),
        root_kp
    ).unwrap();

    match Certificate::from_params(root_cp) {
        Ok(cert) => Ok(cert),
        Err(e) => Err(anyhow!("{}", e.to_string())),
    }
}
