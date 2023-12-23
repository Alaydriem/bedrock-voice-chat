use anyhow::anyhow;
use rocket::time::{Duration, OffsetDateTime};
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

use rcgen::{Certificate, CertificateParams, KeyPair, PKCS_ED25519};
pub(crate) mod auth;
pub(crate) mod config;
pub(crate) mod mc;

pub(crate) fn is_certificate_expiring(certificate_path: &str, player_name: &str) -> bool {
    let player_cert_path_str = format!("{}/{}.crt", &certificate_path, player_name);
    let player_key_path_str = format!("{}/{}.key", &certificate_path, player_name);

    let kp = KeyPair::from_pem(&fs::read_to_string(player_key_path_str).unwrap()).unwrap();
    let cp =
        CertificateParams::from_ca_cert_pem(&fs::read_to_string(player_cert_path_str).unwrap(), kp)
            .unwrap();

    // If the certificate is expiring in 15 days, renew it.
    if cp.not_after
        <= OffsetDateTime::now_utc()
            .checked_sub(Duration::days(-15))
            .unwrap()
    {
        return true;
    }

    return false;
}

/// Creates a new certificate and keypair for the given player from their name to the certificates_path directory
/// This certificate is signed by the root CA for both mTLS and QUIC MoQ Transport
pub(crate) fn create_player_certificate(
    player_name: &str,
    certificate_path: String,
    root_certificate: &Certificate,
) -> Result<Certificate, anyhow::Error> {
    let player_kp = match KeyPair::generate(&PKCS_ED25519) {
        Ok(r) => r,
        Err(_) => return Err(anyhow!("Unable to generate keypair")),
    };

    let mut distinguished_name = rcgen::DistinguishedName::new();
    distinguished_name.push(rcgen::DnType::CommonName, player_name);

    let mut cp = CertificateParams::new(vec![player_name.to_string().clone()]);
    cp.alg = &PKCS_ED25519;
    cp.not_before = OffsetDateTime::now_utc();
    // Certificates are valid for 90 days
    // Having the server advertise the user being connected to the server will automatically issue a new certificate
    // The client will need to retrieve the updated certificate to continue working 15 days or less prior
    // If the client certificate is invalid we bounce them
    // Certificates aren't revoked, the only expire
    cp.not_after = cp.not_before.checked_add(Duration::days(90)).unwrap();
    cp.distinguished_name = distinguished_name;
    cp.key_pair = Some(player_kp);

    let player_certificate = match Certificate::from_params(cp) {
        Ok(c) => c,
        Err(_) => return Err(anyhow!("Unable to generate certificate")),
    };

    // This is the signed player certificate
    let signed_player_certificate = player_certificate
        .serialize_pem_with_signer(&root_certificate)
        .unwrap();

    let key: String = player_certificate.get_key_pair().serialize_pem();

    let player_cert_path_str = format!("{}/{}.crt", &certificate_path, player_name);
    let player_key_path_str = format!("{}/{}.key", &certificate_path, player_name);

    let mut key_file = File::create(player_cert_path_str).unwrap();
    key_file
        .write_all(signed_player_certificate.as_bytes())
        .unwrap();
    let mut cert_file = File::create(player_key_path_str).unwrap();
    cert_file.write_all(key.as_bytes()).unwrap();

    return Ok(player_certificate);
}

/// Retrieves the certificate information for a given player
/// !todo() move this all into a database instead of on-disk files
pub(crate) fn get_certificate_and_key_for_player(
    player_name: &str,
    certificate_path: String,
) -> Result<(String, String), anyhow::Error> {
    let player_certificate_path = format!("{}/{}.crt", &certificate_path, &player_name);
    let player_key_path = format!("{}/{}.key", &certificate_path, &player_name);

    if Path::new(&player_certificate_path).exists() && Path::new(&player_key_path).exists() {
        let cert = fs::read_to_string(&player_certificate_path).unwrap();
        let key = fs::read_to_string(&player_key_path).unwrap();

        return Ok((cert, key));
    } else {
        return Err(anyhow!("Could not retrieve certificate or key for player."));
    }
}
