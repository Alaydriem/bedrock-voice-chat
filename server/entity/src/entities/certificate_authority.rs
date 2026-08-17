use sea_orm::entity::prelude::*;

/// The deployment's certificate authority.
///
/// Exactly one row, id 1. The keypair is the trust anchor for every player certificate ever
/// issued, so it is written once and then only read — replacing it would invalidate every
/// leaf signed by it.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "certificate_authority")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub certificate_pem: String,
    pub key_pem: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
