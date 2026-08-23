use sea_orm::entity::prelude::*;

/// The deployment's ACME account and its current certificate.
///
/// Exactly one row, id 1. The certificate and its key share the row because a pair that
/// spans rows can diverge, and a certificate signed by a key that does not match it fails
/// signature verification rather than chain verification.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "acme_credential")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i32,
    pub account_json: String,
    pub certificate_pem: Option<String>,
    pub key_pem: Option<String>,
    pub directory_url: String,
    pub names: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
