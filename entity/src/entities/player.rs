use sea_orm::entity::prelude::*;
use sea_orm::{ ActiveValue, ActiveModelBehavior };

use common::ncryptflib as ncryptf;

use anyhow::anyhow;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "player")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub gamertag: Option<String>,
    pub gamerpic: Option<String>,
    pub banished: bool,
    pub keypair: Vec<u8>,
    pub signature: Vec<u8>,
    pub created_at: u32,
    pub updated_at: u32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, _insert: bool) -> Result<Self, DbErr>
        where C: ConnectionTrait
    {
        self.updated_at = ActiveValue::Set(
            common::ncryptflib::rocket::Utc::now().timestamp() as u32
        );
        Ok(self)
    }
}

impl Model {
    pub fn get_keypair(&self) -> Result<ncryptf::Keypair, anyhow::Error> {
        let pk = self.keypair[0..32].to_vec();
        let sk = self.keypair[32..64].to_vec();

        match ncryptf::Keypair::from(sk, pk) {
            Ok(kp) => Ok(kp),
            Err(_) => Err(anyhow!("Could not retrieve keypair.")),
        }
    }

    pub fn get_signature(&self) -> Result<ncryptf::Keypair, anyhow::Error> {
        let pk = self.signature[0..32].to_vec();
        let sk = self.signature[32..96].to_vec();

        match ncryptf::Keypair::from(sk, pk) {
            Ok(kp) => Ok(kp),
            Err(_) => Err(anyhow!("Could not retrieve signature keypair.")),
        }
    }
}
