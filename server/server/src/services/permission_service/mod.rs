//! ABAC permission service for evaluating player permissions

use std::collections::HashMap;

use common::structs::permission::Permission;
use entity::player_permission;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

/// Service for evaluating player permissions using ABAC model.
/// Server config provides defaults; per-player overrides stored in `player_permission` table.
pub struct PermissionService {
    defaults: HashMap<String, i32>,
}

impl PermissionService {
    /// Create a new PermissionService with config-defined default permission effects.
    pub fn new(defaults: HashMap<String, i32>) -> Self {
        Self { defaults }
    }

    /// Evaluate a single permission for a player.
    /// Returns true if the player is allowed the given permission.
    ///
    /// Resolution order:
    /// 1. Check `player_permission` table for an override
    /// 2. Fall back to config defaults
    /// 3. If no default exists, deny
    pub async fn evaluate<C: ConnectionTrait>(
        &self,
        conn: &C,
        player_id: i32,
        permission: &Permission,
    ) -> bool {
        let perm_str = permission.as_str().to_string();

        // Check for player-specific override
        let override_result = player_permission::Entity::find()
            .filter(player_permission::Column::PlayerId.eq(player_id))
            .filter(player_permission::Column::Permission.eq(perm_str.clone()))
            .one(conn)
            .await;

        if let Ok(Some(record)) = override_result {
            return record.effect & 1 == 1;
        }

        // Fall back to config defaults
        if let Some(&default_effect) = self.defaults.get(&perm_str) {
            return default_effect & 1 == 1;
        }

        // No default configured: deny
        false
    }

    /// Evaluate all permissions for a player, returning those that are allowed.
    pub async fn evaluate_all<C: ConnectionTrait>(
        &self,
        conn: &C,
        player_id: i32,
    ) -> Vec<Permission> {
        let mut allowed = Vec::new();
        for perm in Permission::all() {
            if self.evaluate(conn, player_id, &perm).await {
                allowed.push(perm);
            }
        }
        allowed
    }
}
