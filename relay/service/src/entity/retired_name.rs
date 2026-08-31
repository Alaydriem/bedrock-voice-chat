use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "retired_name")]
pub struct Model {
    // Every name ever assigned, kept forever. Generation consults this so a name is
    // never offered twice: the previous holder's address is still in operator
    // configuration and in client history, and reassigning it would hand the next
    // holder a publicly trusted certificate for a name other people still resolve.
    #[sea_orm(primary_key, auto_increment = false)]
    pub name: String,
    pub retired_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
