use sea_orm::entity::prelude::*;

/// One issued game-server credential.
///
/// The secret is never stored. `secret_hash` is the lowercase hex SHA-256 of the
/// 32-character secret half of `bvc_<id>_<secret>`, so a database read cannot yield a
/// working token. `revoked_at` is a soft delete: the row survives so an operator can see
/// what was retired and when, and so an id is never reissued.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "game_access_token")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub secret_hash: String,
    pub created_at: i64,
    pub revoked_at: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
