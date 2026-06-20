use anyhow::Result;
use common::ncryptflib as ncryptf;
use common::structs::permission::PermissionEffect;
use entity::player_permission;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveValue, DatabaseConnection, EntityTrait};

pub struct PermissionFixture;

impl PermissionFixture {
    pub async fn upsert(
        db: &DatabaseConnection,
        player_id: i32,
        permission: &str,
        effect: PermissionEffect,
    ) -> Result<()> {
        let now = ncryptf::rocket::Utc::now().timestamp();
        let active = player_permission::ActiveModel {
            id: ActiveValue::NotSet,
            player_id: ActiveValue::Set(player_id),
            permission: ActiveValue::Set(permission.to_string()),
            effect: ActiveValue::Set(effect.to_db()),
            created_at: ActiveValue::Set(now),
        };
        player_permission::Entity::insert(active)
            .on_conflict(
                OnConflict::columns([
                    player_permission::Column::PlayerId,
                    player_permission::Column::Permission,
                ])
                .update_column(player_permission::Column::Effect)
                .to_owned(),
            )
            .exec_without_returning(db)
            .await?;
        Ok(())
    }
}
