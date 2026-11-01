use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "play_zone_state")]
pub struct Model {
    // Always 1: the table holds one row.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i32,
    // The SOA serial. Never goes backwards.
    pub serial: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
