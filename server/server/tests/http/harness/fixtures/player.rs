use anyhow::Result;
use bvc_server_lib::services::CertificateService;
use common::Game;
use common::ncryptflib as ncryptf;
use entity::player;
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection};

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
