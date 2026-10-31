use bvc_server_lib::config::ApplicationConfig;
use bvc_server_lib::services::{
    AuthService, CertificateRevocationService, PlayerIdentityService, PlayerRegistrarService,
};
use common::game_data::Dimension;
use common::players::MinecraftPlayer;
use common::{Coordinate, Game, Orientation, PlayerEnum};
use entity::{player, player_identity};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::sync::Arc;

use super::{CertificateFixture, DatabaseFixture};

/// A BVC server's identity services over a fresh database, driven the way a game server
/// drives them.
///
/// `ingest` runs the calls the position route and the FFI position entry point make on
/// every roster, in their order. `sign_in` is the Xbox login's lookup.
pub struct GameServerReport {
    pub db: DatabaseFixture,
    pub certs: CertificateFixture,
    pub identity: PlayerIdentityService,
    pub registrar: PlayerRegistrarService,
}

impl GameServerReport {
    pub async fn new() -> Self {
        let db = DatabaseFixture::create().await.expect("db fixture");
        let certs = CertificateFixture::create().expect("cert fixture");
        let conn = Arc::new(db.connection.clone());
        let identity = PlayerIdentityService::new(conn.clone());
        let registrar = PlayerRegistrarService::new(conn, certs.service.clone());

        Self {
            db,
            certs,
            identity,
            registrar,
        }
    }

    /// A server restart: fresh services, and so cold caches, over the same database.
    pub fn restart(&mut self) {
        let conn = Arc::new(self.db.connection.clone());
        self.identity = PlayerIdentityService::new(conn.clone());
        self.registrar = PlayerRegistrarService::new(conn, self.certs.service.clone());
    }

    pub fn player(name: &str, alternative: Option<&str>, uuid: Option<&str>) -> PlayerEnum {
        Self::player_at(name, alternative, uuid, 0.0)
    }

    pub fn player_at(
        name: &str,
        alternative: Option<&str>,
        uuid: Option<&str>,
        x: f32,
    ) -> PlayerEnum {
        PlayerEnum::Minecraft(MinecraftPlayer {
            name: name.to_string(),
            coordinates: Coordinate { x, y: 64.0, z: 0.0 },
            orientation: Orientation { x: 0.0, y: 0.0 },
            dimension: Dimension::Overworld,
            whispering: false,
            spectator: false,
            world_uuid: None,
            alternative_identity: alternative.map(String::from),
            player_uuid: uuid.map(String::from),
            relay_world_uuid: None,
            bridged_voice: false,
        })
    }

    /// Runs the roster through the server and returns it as it would be broadcast.
    pub async fn ingest(&self, mut players: Vec<PlayerEnum>) -> Vec<PlayerEnum> {
        let game = Game::Minecraft;
        let claims = self.identity.canonicalize_roster(&mut players, &game).await;
        self.registrar.process_players(&players, game.clone()).await;
        self.identity.record_alias_claims(&claims, &game).await;
        players
    }

    pub async fn sign_in(&self, gamertag: &str) -> Result<String, String> {
        let mut config = ApplicationConfig::default();
        config.server.tls.certs_path = self.certs.certs_path();
        let revocations = CertificateRevocationService::new_shared();

        AuthService::build_login_response(
            &self.db.connection,
            &config.server,
            &self.certs.service,
            None,
            revocations.as_ref(),
            gamertag.to_string(),
            String::new(),
            Game::Minecraft,
        )
        .await
        .map(|response| response.gamertag)
        .map_err(|e| e.to_string())
    }

    pub async fn record(&self, gamertag: &str) -> Option<player::Model> {
        self.registrar
            .find(gamertag, &Game::Minecraft)
            .await
            .expect("find")
    }

    /// The record a platform UUID is bound to, if any.
    pub async fn uuid_owner(&self, uuid: &str) -> Option<i32> {
        player_identity::Entity::find()
            .filter(player_identity::Column::Alias.eq(uuid))
            .filter(player_identity::Column::AliasType.eq("platform_uuid"))
            .filter(player_identity::Column::Game.eq(Game::Minecraft))
            .one(&self.db.connection)
            .await
            .expect("query")
            .map(|row| row.player_id)
    }

    pub async fn resolve(&self, name: &str) -> String {
        self.identity.resolve_name(name, &Game::Minecraft).await
    }
}
