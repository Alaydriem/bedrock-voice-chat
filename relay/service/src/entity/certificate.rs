use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "certificate")]
pub struct Model {
    // The name the certificate was issued for. Keying by it means a hostname change
    // does not silently serve the old name's certificate.
    #[sea_orm(primary_key, auto_increment = false)]
    pub hostname: String,
    // Leaf first, then the issuer chain, exactly as the certificate authority returned
    // it. Stored whole: splitting it would mean reassembling it in the right order on
    // every read.
    pub chain_pem: String,
    pub key_pem: String,
    pub issued_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
