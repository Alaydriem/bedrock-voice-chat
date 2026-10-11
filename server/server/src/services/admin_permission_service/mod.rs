use std::sync::Arc;

use common::request::admin::AdminPermissionRequest;
use common::structs::permission::{AdminAction, Permission, PermissionEffect};
use entity::player;
use sea_orm::DatabaseConnection;

use crate::services::{PermissionService, PlayerRegistrarService};

mod outcome;

pub use outcome::AdminPermissionOutcome;

/// Changes a player's `admin` permission for an operator who already controls the server,
/// such as the console of the embedding mod.
#[derive(Clone)]
pub struct AdminPermissionService {
    db: Arc<DatabaseConnection>,
    registrar: PlayerRegistrarService,
}

impl AdminPermissionService {
    const MAX_GAMERTAG_LEN: usize = 64;

    pub fn new(db: Arc<DatabaseConnection>, registrar: PlayerRegistrarService) -> Self {
        Self { db, registrar }
    }

    /// The gamertag is matched verbatim, case included.
    ///
    /// Grant registers a player that does not exist yet. Deny and revoke never do: a player
    /// row is what lets a gamertag sign in, so creating one to refuse admin would hand out a
    /// login.
    pub async fn apply(
        &self,
        request: &AdminPermissionRequest,
    ) -> Result<AdminPermissionOutcome, anyhow::Error> {
        let trimmed = request.gamertag.trim();
        if trimmed.is_empty() || trimmed.len() > Self::MAX_GAMERTAG_LEN {
            anyhow::bail!("gamertag must be 1-{} characters", Self::MAX_GAMERTAG_LEN);
        }

        let Some(player) = self.target(request).await? else {
            return Ok(AdminPermissionOutcome::PlayerNotFound);
        };

        let conn = self.db.as_ref();
        let admin = Permission::Admin.as_str();
        let outcome = match request.action {
            AdminAction::Grant => {
                PermissionService::set_override(conn, player.id, admin, PermissionEffect::Allow)
                    .await?;
                AdminPermissionOutcome::Applied
            }
            AdminAction::Deny => {
                PermissionService::set_override(conn, player.id, admin, PermissionEffect::Deny)
                    .await?;
                AdminPermissionOutcome::Applied
            }
            AdminAction::Revoke => {
                if PermissionService::clear_override(conn, player.id, admin).await? {
                    AdminPermissionOutcome::Applied
                } else {
                    AdminPermissionOutcome::NothingToRevoke
                }
            }
        };

        Ok(outcome)
    }

    async fn target(
        &self,
        request: &AdminPermissionRequest,
    ) -> Result<Option<player::Model>, anyhow::Error> {
        match request.action {
            AdminAction::Grant => Ok(Some(
                self.registrar
                    .create(&request.gamertag, &request.game, None)
                    .await?,
            )),
            AdminAction::Revoke | AdminAction::Deny => {
                Ok(self.registrar.find(&request.gamertag, &request.game).await?)
            }
        }
    }
}
