use std::{path::Path, fs::{self, File}, io::Write};

use common::{
    pool::redis::RedisDb,
    rocket::http::Status,
    rocket::serde::json::Json,
    rocket_db_pools::Connection as RedisConnection,
};

use anyhow::anyhow;

#[allow(unused_imports)] // for rust-analyzer
use common::rocket_db_pools::deadpool_redis::redis::AsyncCommands;
use rcgen::{KeyPair, CertificateParams, Certificate, PKCS_ED25519};
use rocket::{State, time::OffsetDateTime};
use crate::{rs::guards::MCAccessToken, config::ApplicationConfigServer};

/// Stores player position data and online status from Minecraft Bedrock into Redis
#[post("/", data = "<positions>")]
pub fn position(
    // Guard the request so it's only accepted if we have a valid access token
    _access_token: MCAccessToken,
    // Data is to be stored in Redis
    rdb: RedisConnection<RedisDb>,
    // The player position data
    positions: Json<Vec::<common::Player>>,
    // Certificates path from the State Configuration
    config: &State<ApplicationConfigServer>
) -> Status {

    // Generate the paths
    let certificate_path = config.tls.certs_path.clone();
    let root_ca_path_str = format!("{}/{}", &certificate_path, "ca.crt");
    let root_ca_key_path_str = format!("{}/{}", &certificate_path, "ca.key");
    let root_kp = KeyPair::from_pem(&fs::read_to_string(root_ca_key_path_str).unwrap()).unwrap();
    let root_cp = CertificateParams::from_ca_cert_pem(&fs::read_to_string(root_ca_path_str).unwrap(), root_kp).unwrap();
    let root_certificate = Certificate::from_params(root_cp).unwrap();

    // Iterate through each of the players
    for player in positions.0 {
        let player_name = player.name;
        let player_certificate_path = format!("{}/{}.crt", &certificate_path, &player_name);

        // If the certificate for the player doesn't exist, create it.
        if !Path::new(&player_certificate_path).exists() {
            match create_player_certificate(player_name, certificate_path.clone(), &root_certificate) {
                Ok(_) => {},
                Err(e) => {
                    tracing::error!("{}", e.to_string());
                    return Status::InternalServerError;
                }
            };
        }
    }
    return Status::Ok;
}


/// Creates a new certificate and keypair for the given player from their name to the certificates_path directory
/// This certificate is signed by the root CA for both mTLS and QUIC MoQ Transport
fn create_player_certificate(player_name: String, certificate_path: String, root_certificate: &Certificate) -> Result<Certificate, anyhow::Error> {
    let player_kp = match KeyPair::generate(&PKCS_ED25519) {
        Ok(r) => r,
        Err(_) => return Err(anyhow!("Unable to generate keypair"))
    };

    let mut distinguished_name = rcgen::DistinguishedName::new();
    distinguished_name.push(rcgen::DnType::CommonName, &player_name);

    let mut cp = CertificateParams::new(vec![player_name.clone()]);
    cp.alg = &PKCS_ED25519;
    cp.not_before = OffsetDateTime::now_utc();
    cp.distinguished_name = distinguished_name;
    cp.key_pair = Some(player_kp);

    let player_certificate = match Certificate::from_params(cp) {
        Ok(c) => c,
        Err(_) => return Err(anyhow!("Unable to generate certificate"))
    };
    
    // This is the signed player certificate
    let signed_player_certificate = player_certificate.serialize_pem_with_signer(&root_certificate).unwrap();

    let key: String = player_certificate.get_key_pair().serialize_pem();

    let player_cert_path_str = format!("{}/{}.crt", &certificate_path, &player_name);
    let player_key_path_str = format!("{}/{}.key", &certificate_path, &player_name);

    let mut key_file = File::create(player_cert_path_str).unwrap();
    key_file.write_all(signed_player_certificate.as_bytes()).unwrap();
    let mut cert_file = File::create(player_key_path_str).unwrap();
    cert_file.write_all(key.as_bytes()).unwrap();

    return Ok(player_certificate);
}