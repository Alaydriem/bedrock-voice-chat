use common::sea_orm::{self, entity::prelude::*, ActiveValue, DeriveEntityModel};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
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

impl ActiveModelBehavior for ActiveModel {
    fn before_save(mut self, _insert: bool) -> Result<Self, DbErr> {
        self.updated_at =
            ActiveValue::Set(common::ncryptflib::rocket::Utc::now().timestamp() as u32);
        Ok(self)
    }
}
