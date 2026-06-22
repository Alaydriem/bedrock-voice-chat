use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, ActiveValue};

use common::ncryptflib as ncryptf;

use x509_parser::prelude::*;

use ::time::{Duration, OffsetDateTime};
use anyhow::anyhow;

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

use super::player_auth_code;

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "player_auth_code::Entity")]
    PlayerAuthCode,
}

impl Related<player_auth_code::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PlayerAuthCode.def()
    }
}

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

    /// Returns true if the certificate in storage is expiring within 15 days
    pub fn is_certificate_expiring(&self) -> Result<bool, anyhow::Error> {
        let (_, pem) = parse_x509_pem(self.certificate.as_bytes())
            .map_err(|e| anyhow!("Failed to parse certificate PEM: {}", e))?;
        let (_, cert) = X509Certificate::from_der(&pem.contents)
            .map_err(|e| anyhow!("Failed to parse X.509 certificate: {}", e))?;

        let not_after = cert.validity().not_after.to_datetime();
        let renewal_threshold = OffsetDateTime::now_utc() + Duration::days(15);
        Ok(not_after <= renewal_threshold)
    }

    /// Returns true if the certificate CN uses the legacy format (no game prefix)
    pub fn has_legacy_certificate_cn(&self, game: &common::Game) -> bool {
        let expected_prefix = format!("{}:", game.as_str());
        let Ok((_, pem)) = parse_x509_pem(self.certificate.as_bytes()) else {
            return true;
        };
        let Ok((_, cert)) = X509Certificate::from_der(&pem.contents) else {
            return true;
        };
        let cn = cert
            .subject()
            .iter_common_name()
            .next()
            .and_then(|attr| attr.as_str().ok());
        match cn {
            Some(cn) => !cn.starts_with(&expected_prefix),
            None => true,
        }
    }
}
