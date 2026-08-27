use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "dns_record")]
pub struct Model {
    // Cloudflare's own record id. Cleanup deletes by id rather than by matching
    // content, so a value written twice is still removed exactly once, and an
    // interrupted publish leaves no record nobody can identify.
    #[sea_orm(primary_key, auto_increment = false)]
    pub record_id: String,
    pub name: String,
    pub record_type: String,
    pub content: String,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
