use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "issuance_log")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    // True when this issuance replaced an existing certificate for the same name.
    // Renewals are the exempt category at the certificate authority, so the weekly
    // budget counts only the rows where this is false.
    pub is_renewal: bool,
    pub issued_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
