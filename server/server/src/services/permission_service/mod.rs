use std::collections::HashMap;

use common::structs::permission::{Permission, PermissionEffect};
use entity::player_permission;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, ModelTrait,
    QueryFilter,
};

#[derive(Debug, thiserror::Error)]
pub enum PermissionServiceError {
    #[error("unknown permission: {0}")]
    UnknownPermission(String),
    #[error("database error: {0}")]
    Database(#[from] sea_orm::DbErr),
}

pub struct PermissionService {
    defaults: HashMap<String, bool>,
}

impl PermissionService {
    pub fn new(defaults: HashMap<String, bool>) -> Self {
        Self { defaults }
    }

    pub async fn evaluate<C: ConnectionTrait>(
        &self,
        conn: &C,
        player_id: i32,
        permission: &Permission,
    ) -> bool {
        let perm_str = permission.as_str().to_string();

        let override_result = player_permission::Entity::find()
            .filter(player_permission::Column::PlayerId.eq(player_id))
            .filter(player_permission::Column::Permission.eq(perm_str.clone()))
            .one(conn)
            .await;

        if let Ok(Some(record)) = override_result {
            return record.effect & 1 == 1;
        }

        if let Some(&default_val) = self.defaults.get(&perm_str) {
            return default_val;
        }

        false
    }

    pub async fn evaluate_all<C: ConnectionTrait>(
        &self,
        conn: &C,
        player_id: i32,
    ) -> Vec<Permission> {
        let overrides: HashMap<String, i32> = player_permission::Entity::find()
            .filter(player_permission::Column::PlayerId.eq(player_id))
            .all(conn)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|p| (p.permission, p.effect))
            .collect();

        Permission::all()
            .into_iter()
            .filter(|perm| {
                let key = perm.as_str().to_string();
                if let Some(&effect) = overrides.get(&key) {
                    effect & 1 == 1
                } else {
                    *self.defaults.get(&key).unwrap_or(&false)
                }
            })
            .collect()
    }

    pub async fn set_override<C: ConnectionTrait>(
        conn: &C,
        player_id: i32,
        permission: &str,
        effect: PermissionEffect,
    ) -> Result<(), PermissionServiceError> {
        if Permission::from_str(permission).is_none() {
            return Err(PermissionServiceError::UnknownPermission(
                permission.to_string(),
            ));
        }

        let existing = player_permission::Entity::find()
            .filter(player_permission::Column::PlayerId.eq(player_id))
            .filter(player_permission::Column::Permission.eq(permission.to_string()))
            .one(conn)
            .await?;

        match existing {
            Some(record) => {
                let mut active: player_permission::ActiveModel = record.into();
                active.effect = ActiveValue::Set(effect.to_db());
                active.update(conn).await?;
            }
            None => {
                let now = common::ncryptflib::rocket::Utc::now().timestamp();
                let active = player_permission::ActiveModel {
                    id: ActiveValue::NotSet,
                    player_id: ActiveValue::Set(player_id),
                    permission: ActiveValue::Set(permission.to_string()),
                    effect: ActiveValue::Set(effect.to_db()),
                    created_at: ActiveValue::Set(now),
                };
                active.insert(conn).await?;
            }
        }

        Ok(())
    }

    pub async fn clear_override<C: ConnectionTrait>(
        conn: &C,
        player_id: i32,
        permission: &str,
    ) -> Result<bool, PermissionServiceError> {
        let existing = player_permission::Entity::find()
            .filter(player_permission::Column::PlayerId.eq(player_id))
            .filter(player_permission::Column::Permission.eq(permission.to_string()))
            .one(conn)
            .await?;

        match existing {
            Some(record) => {
                record.delete(conn).await?;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    pub async fn list_overrides<C: ConnectionTrait>(
        conn: &C,
        player_id: i32,
    ) -> Result<Vec<(String, PermissionEffect)>, PermissionServiceError> {
        let records = player_permission::Entity::find()
            .filter(player_permission::Column::PlayerId.eq(player_id))
            .all(conn)
            .await?;

        Ok(records
            .into_iter()
            .map(|r| (r.permission, PermissionEffect::from_db(r.effect)))
            .collect())
    }
}
