use sea_orm::entity::prelude::*;
use sea_orm::{ActiveValue, ActiveModelBehavior};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "player")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub gamertag: Option<String>,
    pub gamerpic: Option<String>,
    pub banished: bool,
    pub created_at: u32,
    pub updated_at: u32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, _insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        self.updated_at =
            ActiveValue::Set(common::ncryptflib::rocket::Utc::now().timestamp() as u32);
        Ok(self)
    }
}
