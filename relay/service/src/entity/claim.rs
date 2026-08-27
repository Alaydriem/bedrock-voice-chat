use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "claim")]
pub struct Model {
    // Random and unguessable. It travels in a redirect URL, so anything sequential or
    // derived from the token would let one member take another's.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub token: String,
    pub expires_at: i64,
    pub consumed_at: Option<i64>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
