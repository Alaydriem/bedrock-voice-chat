//! Certificate service for player authentication

use std::fs;
use std::sync::Arc;

use anyhow::anyhow;
use common::Game;
use rcgen::{
    Certificate, CertificateParams, DistinguishedName, ExtendedKeyUsagePurpose, Issuer, KeyPair,
    SanType,
};
use time::{Duration, OffsetDateTime};

/// Service for certificate operations for player authentication.
/// Caches the root CA issuer to avoid repeated file I/O.
pub struct CertificateService {
    issuer: Issuer<'static, KeyPair>,
}

impl CertificateService {
    /// Create a new CertificateService by loading the root CA from the given path.
    ///
    /// # Arguments
    /// * `certs_path` - Path to the certificates directory containing ca.crt and ca.key
    pub fn new(certs_path: &str) -> Result<Self, anyhow::Error> {
        let issuer = Self::load_root_ca(certs_path)?;
        Ok(Self { issuer })
    }

    /// Create a new CertificateService wrapped in Arc for sharing between components.
    pub fn new_shared(certs_path: &str) -> Result<Arc<Self>, anyhow::Error> {
        Ok(Arc::new(Self::new(certs_path)?))
    }

    /// Sign a new player certificate using the cached root CA.
    ///
    /// # Arguments
    /// * `player_name` - The player's name (used as Common Name in the certificate)
    /// * `game` - The game type, embedded in the CN as "game:player_name"
    ///
    /// # Returns
    /// A tuple of (Certificate, KeyPair) for the player
    pub fn sign_player_cert(
        &self,
        player_name: &str,
        game: &Game,
    ) -> Result<(Certificate, KeyPair), anyhow::Error> {
        // Display uses Debug format ("Hytale"), but DB stores lowercase ("hytale")
        let cn = format!("{}:{}", format!("{}", game).to_lowercase(), player_name);
        let mut dn = DistinguishedName::new();
        dn.push(rcgen::DnType::CommonName, cn);

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
            SanType::DnsName(String::from("localhost").try_into()?),
            SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))),
            SanType::IpAddress(std::net::IpAddr::V6(std::net::Ipv6Addr::new(
                0, 0, 0, 0, 0, 0, 0, 1,
            ))),
        ];

        let key_pair = KeyPair::generate()?;
        let cert = params.signed_by(&key_pair, &self.issuer);
        match cert {
            Ok(cert) => Ok((cert, key_pair)),
            Err(_) => Err(anyhow!("Unable to generate certificate")),
        }
    }

    /// Load the root CA as an Issuer from disk.
    fn load_root_ca(certificate_path: &str) -> Result<Issuer<'static, KeyPair>, anyhow::Error> {
        let root_ca_path_str = format!("{}/{}", certificate_path, "ca.crt");
        let root_ca_key_path_str = format!("{}/{}", certificate_path, "ca.key");
        let root_kp = KeyPair::from_pem(&fs::read_to_string(root_ca_key_path_str)?)?;
        let issuer = Issuer::from_ca_cert_pem(
            &fs::read_to_string(root_ca_path_str)?,
            root_kp,
        )?;

        Ok(issuer)
    }
}
