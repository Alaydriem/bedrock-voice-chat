use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, ActiveValue};

use super::player;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "player_auth_code")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub code: String,
    pub player_id: i32,
    pub expires_at: u32,
    pub used: bool,
    pub created_at: u32,
    pub updated_at: u32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "player::Entity",
        from = "Column::PlayerId",
        to = "player::Column::Id"
    )]
    Player,
}

impl Related<player::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Player.def()
    }
}

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
