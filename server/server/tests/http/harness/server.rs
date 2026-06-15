//! `TestServer` — the entry point each integration test uses.
//!
//! Construction order matters and is stable across tests:
//! 1. TempDir for certs / assets / sqlite db
//! 2. CA generated in TempDir
//! 3. Server TLS leaf signed by CA (so reqwest trusts the server via add_root_certificate)
//! 4. SQLite connected, migrations run
//! 5. Admin player + admin permission inserted
//! 6. Admin's client cert signed (used by `admin_client()`)
//! 7. Rocket spawned on a random port; we poll until reachable

use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use bvc_server_lib::config::ApplicationConfig;
use bvc_server_lib::services::CertificateService;
use common::structs::permission::PermissionEffect;
use common::Game;
use entity::player;
use migration::{Migrator, MigratorTrait};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectOptions, Database, DatabaseConnection,
    EntityTrait, QueryFilter,
};
use tempfile::TempDir;

use super::ca::GeneratedCa;
use super::fixtures::{PermissionFixture, PlayerFixture};
use super::http_client::MtlsClient;
use super::rocket_harness::RocketHarness;

pub const ADMIN_GAMERTAG: &str = "RootAdmin";
pub const ADMIN_GAME: Game = Game::Minecraft;

pub struct TestServer {
    pub base_url: String,
    pub ca_pem: String,
    pub admin_cert: String,
    pub admin_key: String,
    pub admin_id: i32,
    pub cert_service: Arc<CertificateService>,
    pub db: DatabaseConnection,
    _tmp: TempDir,
    _server_task: tokio::task::JoinHandle<()>,
}

impl TestServer {
    pub async fn start() -> Result<Self> {
        Self::start_with_relay(false).await
    }

    pub async fn start_with_relay(relay_enabled: bool) -> Result<Self> {
        // rustls crypto provider: install once per process; ignore re-install error.
        let _ = common::s2n_quic::provider::tls::rustls::rustls::crypto::aws_lc_rs::default_provider()
            .install_default();

        let tmp = TempDir::new()?;
        let certs_path = tmp.path().join("certs");
        std::fs::create_dir_all(&certs_path)?;
        let assets_path = tmp.path().join("assets");
        std::fs::create_dir_all(&assets_path)?;
        let db_path = tmp.path().join("test.sqlite3");
        std::fs::File::create(&db_path)?;

        let ca = GeneratedCa::generate(&[
            "localhost".into(),
            "127.0.0.1".into(),
        ])?;
        std::fs::write(certs_path.join("ca.crt"), &ca.cert_pem)?;
        std::fs::write(certs_path.join("ca.key"), &ca.key_pem)?;

        let cert_service = CertificateService::new_shared(certs_path.to_str().unwrap())?;

        // Server TLS leaf cert. Signed by the same CA so the test's reqwest client
        // (which trusts the CA via add_root_certificate) accepts it.
        let (server_cert, server_key) =
            cert_service.sign_player_cert("server", &Game::Minecraft)?;
        let server_cert_path = certs_path.join("server.crt");
        let server_key_path = certs_path.join("server.key");
        std::fs::write(&server_cert_path, server_cert.pem())?;
        std::fs::write(&server_key_path, server_key.serialize_pem())?;

        let port = pick_free_port()?;
        let base_url = format!("https://127.0.0.1:{}", port);

        let dsn = format!("sqlite://{}", db_path.display());
        let mut opts = ConnectOptions::new(dsn);
        opts.max_connections(8)
            .min_connections(1)
            .connect_timeout(Duration::from_secs(3))
            .idle_timeout(Duration::from_secs(30))
            .sqlx_logging(false);
        let db = Database::connect(opts).await?;
        Migrator::up(&db, None).await?;

        let admin = PlayerFixture::insert(&db, &cert_service, ADMIN_GAMERTAG, &ADMIN_GAME).await?;
        PermissionFixture::upsert(&db, admin.id, "admin", PermissionEffect::Allow).await?;
        let admin_id = admin.id;

        let (admin_cert_obj, admin_key_obj) =
            cert_service.sign_player_cert(ADMIN_GAMERTAG, &ADMIN_GAME)?;
        let admin_cert = admin_cert_obj.pem();
        let admin_key = admin_key_obj.serialize_pem();

        let mut config = ApplicationConfig::default();
        config.server.features.code_login = true;
        config.server.features.relay.enabled = relay_enabled;
        config.database.scheme = "sqlite".into();
        config.database.database = db_path.to_string_lossy().into_owned();
        config.server.port = port as u32;
        config.server.listen = "127.0.0.1".to_string();
        config.server.tls.certificate = server_cert_path.to_string_lossy().into_owned();
        config.server.tls.key = server_key_path.to_string_lossy().into_owned();
        config.server.tls.certs_path = certs_path.to_string_lossy().into_owned();
        config.server.assets_path = assets_path.to_string_lossy().into_owned();

        let server_task = RocketHarness::launch(config, cert_service.clone()).await?;

        // Poll until the server accepts connections (the introspect probe expects 401
        // because the probe client has no client cert; we treat any HTTP response as ready).
        let probe = MtlsClient::no_identity(&ca.cert_pem)?;
        let probe_url = format!("{}/api/auth/introspect", base_url);
        wait_for_ready(&probe, &probe_url).await?;

        Ok(TestServer {
            base_url,
            ca_pem: ca.cert_pem,
            admin_cert,
            admin_key,
            admin_id,
            cert_service,
            db,
            _tmp: tmp,
            _server_task: server_task,
        })
    }

    pub fn admin_client(&self) -> Result<reqwest::Client> {
        MtlsClient::with_identity(&self.ca_pem, &self.admin_cert, &self.admin_key)
    }

    pub fn noauth_client(&self) -> Result<reqwest::Client> {
        MtlsClient::no_identity(&self.ca_pem)
    }

    pub async fn issue_player(&self, gamertag: &str, game: &Game) -> Result<(String, String)> {
        let _ = PlayerFixture::insert(&self.db, &self.cert_service, gamertag, game).await?;
        let (c, k) = self.cert_service.sign_player_cert(gamertag, game)?;
        Ok((c.pem(), k.serialize_pem()))
    }

    pub async fn mark_banished(&self, gamertag: &str, game: &Game, banished: bool) -> Result<()> {
        let p = player::Entity::find()
            .filter(player::Column::Gamertag.eq(gamertag))
            .filter(player::Column::Game.eq(game.clone()))
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow!("player {} not found", gamertag))?;
        let mut active: player::ActiveModel = p.into();
        active.banished = ActiveValue::Set(banished);
        active.update(&self.db).await?;
        Ok(())
    }
}

fn pick_free_port() -> Result<u16> {
    let l = TcpListener::bind("127.0.0.1:0")?;
    Ok(l.local_addr()?.port())
}

async fn wait_for_ready(client: &reqwest::Client, url: &str) -> Result<()> {
    for _ in 0..50 {
        match client.get(url).send().await {
            Ok(_) => return Ok(()),
            Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    }
    Err(anyhow!("server did not become ready within timeout"))
}
