use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "enrollment_token")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub token: String,
    // Bound at issuance, not at redemption. The web UI knows who asked; the iroh
    // link that redeems knows only a node key.
    pub discord_user_id: String,
    pub expires_at: i64,
    pub redeemed_at: Option<i64>,
    pub redeemed_by_node_id: Option<String>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
