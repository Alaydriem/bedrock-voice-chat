use sea_orm::entity::prelude::*;

/// A peer authorized by redeeming a pairing code.
///
/// `worlds` and `capabilities` are comma-separated rather than a serde envelope: both are
/// read by a person debugging a grant, and a column carrying a serialized Rust type is
/// unreadable by anything but this exact struct definition.
///
/// An empty `worlds` is a filter that narrows nothing, which is what lets the peer's own
/// declaration stand.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "peer_grant")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub node_id: String,
    pub label: String,
    pub worlds: String,
    pub capabilities: String,
    pub paired_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
