use sea_orm::entity::prelude::*;

/// A single opaque server secret, keyed by name.
///
/// Holds the values that have no correlated partner: the Minecraft access token and the
/// relay node key. Material that must correspond — a certificate and its key — belongs in a
/// table with both in one row, never two rows here.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "server_secret")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub name: String,
    pub value: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
