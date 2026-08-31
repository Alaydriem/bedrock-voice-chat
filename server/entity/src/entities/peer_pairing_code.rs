use sea_orm::entity::prelude::*;

/// One minted pairing code, held as the digest of the plaintext an operator was given.
///
/// `consumed_at` and the grant row are written in one transaction. A redemption that
/// writes the grant without stamping this column leaves the code live, and single-use is
/// defeated with nothing to show for it.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "peer_pairing_code")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub code_digest: String,
    pub label: String,
    pub expires_at: i64,
    pub consumed_at: Option<i64>,
    pub attempts: i32,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
