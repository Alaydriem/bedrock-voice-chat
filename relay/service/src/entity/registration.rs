use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "registration")]
pub struct Model {
    // The iroh public key, hex encoded. Permanent: an operator who loses it gets a
    // new registration and a new name, and the old name is retired rather than
    // transferred.
    #[sea_orm(primary_key, auto_increment = false)]
    pub node_id: String,
    #[sea_orm(unique)]
    pub name: String,
    #[sea_orm(unique)]
    pub discord_user_id: String,
    pub state: String,
    pub declared_address: Option<String>,
    pub address_verified_at: Option<i64>,
    pub entitlement_checked_at: Option<i64>,
    pub entitlement_ok: bool,
    pub validated_at: Option<i64>,
    pub validation_failures: i32,
    pub created_at: i64,
    pub suspended_at: Option<i64>,
    pub retired_at: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
