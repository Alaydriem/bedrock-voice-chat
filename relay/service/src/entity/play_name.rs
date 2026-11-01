use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "play_name")]
pub struct Model {
    // The iroh public key, hex encoded.
    #[sea_orm(primary_key, auto_increment = false)]
    pub node_id: String,
    // The bare label. A released name keeps its row so a label is never issued twice.
    #[sea_orm(unique)]
    pub name: String,
    // The address the claim arrived from; the rate limit counts by it.
    pub claimed_from: String,
    pub declared_address: Option<String>,
    pub declared_port: Option<i32>,
    // Set only after a probe of the declared address succeeded.
    pub published_address: Option<String>,
    pub verified_at: Option<i64>,
    pub checkin_nonce: Option<String>,
    pub checked_in_at: Option<i64>,
    pub created_at: i64,
    pub released_at: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
