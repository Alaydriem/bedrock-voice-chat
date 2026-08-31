use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "node_key")]
pub struct Model {
    // Named rather than a single implicit row, so a second durable secret becomes
    // another row instead of another table.
    #[sea_orm(primary_key, auto_increment = false)]
    pub name: String,
    // Hex, because the value is 32 raw bytes and every backend here stores text
    // identically while only some store blobs identically.
    pub value: String,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
