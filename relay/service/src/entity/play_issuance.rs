use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "play_issuance")]
pub struct Model {
    // One row per certificate request admitted to the CA; the per-node daily limit
    // counts them.
    #[sea_orm(primary_key)]
    pub id: i64,
    pub node_id: String,
    pub issued_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
