use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, ActiveValue};

use common::ncryptflib as ncryptf;

use anyhow::anyhow;
use x509_parser::pem::parse_x509_pem;
use x509_parser::prelude::{FromDer, X509Certificate};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "player")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub gamertag: Option<String>,
    pub gamerpic: Option<String>,
    pub certificate: String,
    pub certificate_key: String,
    pub banished: bool,
    pub keypair: Vec<u8>,
    pub signature: Vec<u8>,
    pub created_at: u32,
    pub updated_at: u32,
    pub game: common::Game,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, _insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        self.updated_at =
            ActiveValue::Set(common::ncryptflib::rocket::Utc::now().timestamp() as u32);
        Ok(self)
    }
}

impl Model {
    /// Returns the ncryptf keypair
    pub fn get_keypair(&self) -> Result<ncryptf::Keypair, anyhow::Error> {
        let pk = self.keypair[0..32].to_vec();
        let sk = self.keypair[32..64].to_vec();

        match ncryptf::Keypair::from(sk, pk) {
            Ok(kp) => Ok(kp),
            Err(_) => Err(anyhow!("Could not retrieve keypair.")),
        }
    }

    /// Returns the ncryptf signature
    pub fn get_signature(&self) -> Result<ncryptf::Keypair, anyhow::Error> {
        let pk = self.signature[0..32].to_vec();
        let sk = self.signature[32..96].to_vec();

        match ncryptf::Keypair::from(sk, pk) {
            Ok(kp) => Ok(kp),
            Err(_) => Err(anyhow!("Could not retrieve signature keypair.")),
        }
    }

    /// Extract the Common Name (CN) from the stored certificate PEM.
    pub fn get_certificate_cn(&self) -> Result<String, anyhow::Error> {
        let (_, pem) = parse_x509_pem(self.certificate.as_bytes())
            .map_err(|e| anyhow!("Failed to parse certificate PEM: {}", e))?;
        let (_, cert) = X509Certificate::from_der(&pem.contents)
            .map_err(|e| anyhow!("Failed to parse certificate DER: {}", e))?;

        // OID 2.5.4.3 = Common Name
        let cn_oid = x509_parser::oid_registry::OID_X509_COMMON_NAME;
        for rdn in cert.subject().iter() {
            for attr in rdn.iter() {
                if attr.attr_type() == &cn_oid {
                    return attr
                        .attr_value()
                        .as_str()
                        .map(|s| s.to_string())
                        .map_err(|e| anyhow!("Failed to read CN: {}", e));
                }
            }
        }

        Err(anyhow!("Certificate has no Common Name"))
    }

    /// Returns true if the certificate in storage is expiring within 15 days.
    pub fn is_certificate_expiring(&self) -> Result<bool, anyhow::Error> {
        let (_, pem) = parse_x509_pem(self.certificate.as_bytes())
            .map_err(|e| anyhow!("Failed to parse certificate PEM: {}", e))?;
        let (_, cert) = X509Certificate::from_der(&pem.contents)
            .map_err(|e| anyhow!("Failed to parse certificate DER: {}", e))?;

        let not_after_epoch = cert.validity().not_after.timestamp();
        let now_epoch = common::ncryptflib::rocket::Utc::now().timestamp();
        let fifteen_days_secs: i64 = 15 * 24 * 60 * 60;

        Ok(not_after_epoch <= now_epoch + fifteen_days_secs)
    }

    /// Returns true if the certificate needs re-issuance.
    /// A certificate needs re-issuance if it is expiring or uses the old CN
    /// format (gamertag only, without the "game:" prefix).
    pub fn needs_certificate_reissue(&self) -> bool {
        // Check CN format first (cheap string check)
        match self.get_certificate_cn() {
            Ok(cn) => {
                if !cn.contains(':') {
                    return true;
                }
            }
            Err(_) => return true,
        }

        // Check expiry
        self.is_certificate_expiring().unwrap_or(true)
    }
}
