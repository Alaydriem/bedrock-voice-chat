use std::collections::HashMap;

use common::structs::permission::Permission;
use entity::player_permission;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

pub struct PermissionService {
    defaults: HashMap<String, i32>,
}

impl PermissionService {
    pub fn new(defaults: HashMap<String, i32>) -> Self {
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

        if let Some(&default_effect) = self.defaults.get(&perm_str) {
            return default_effect & 1 == 1;
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
                let effect = overrides
                    .get(&key)
                    .copied()
                    .unwrap_or_else(|| *self.defaults.get(&key).unwrap_or(&0));
                effect & 1 == 1
            })
            .collect()
    }
}
