//! DB-direct insert helpers, used to seed the fixture before each test.
//!
//! These mirror the production paths (`PlayerRegistrarService::create_player`,
//! `PermissionService::set_override`) but bypass the HTTP layer because the tests
//! are exercising the routes themselves and need a known starting state.

use anyhow::Result;
use bvc_server_lib::services::CertificateService;
use common::ncryptflib as ncryptf;
use common::structs::permission::PermissionEffect;
use common::Game;
use entity::{player, player_permission};
use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, EntityTrait};

pub struct PlayerFixture;

impl PlayerFixture {
    pub async fn insert(
        db: &DatabaseConnection,
        cert_service: &CertificateService,
        gamertag: &str,
        game: &Game,
    ) -> Result<player::Model> {
        let kp = ncryptf::Keypair::new();
        let signature = ncryptf::Signature::new();
        let mut kpv = Vec::<u8>::new();
        kpv.extend_from_slice(&kp.get_public_key());
        kpv.extend_from_slice(&kp.get_secret_key());
        let mut sgv = Vec::<u8>::new();
        sgv.extend_from_slice(&signature.get_public_key());
        sgv.extend_from_slice(&signature.get_secret_key());

        let (cert, key) = cert_service.sign_player_cert(gamertag, game)?;
        let now = ncryptf::rocket::Utc::now().timestamp() as u32;
        let active = player::ActiveModel {
            id: ActiveValue::NotSet,
            gamertag: ActiveValue::Set(Some(gamertag.to_string())),
            gamerpic: ActiveValue::Set(None),
            certificate: ActiveValue::Set(cert.pem()),
            certificate_key: ActiveValue::Set(key.serialize_pem()),
            banished: ActiveValue::Set(false),
            keypair: ActiveValue::Set(kpv),
            signature: ActiveValue::Set(sgv),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
            game: ActiveValue::Set(game.clone()),
        };
        Ok(active.insert(db).await?)
    }
}

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
