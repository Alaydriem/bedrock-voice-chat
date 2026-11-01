//! A real `RocketManager` running as the plain-HTTP mod listener.
//!
//! Production reaches this listener through the demultiplexer's loopback branch. The tests
//! dial it directly, the way `TestServer` dials the TLS listener, so what they observe is the
//! routing and guards of the listener itself.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use bvc_server_lib::config::ApplicationConfig;
use bvc_server_lib::demux::ApiBind;
use bvc_server_lib::http::manager::{RocketListener, RocketManager};
use bvc_server_lib::services::{
    AccessTokenService, AudioPlaybackService, BedrockEventService, CertificateRevocationService,
    CertificateService, ChatService, CurrentNonce, MetricsService, PlayerIdentityService,
    PlayerRegistrarService,
};
use bvc_server_lib::stream::quic::{CacheManager, WebhookReceiver};
use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database};
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use super::ca::GeneratedCa;

pub const LOOPBACK_TOKEN: &str = "loopback-test-token";

pub struct LoopbackServer {
    pub base_url: String,
    _tmp: TempDir,
    _task: tokio::task::JoinHandle<()>,
}

impl LoopbackServer {
    pub async fn start() -> Result<Self> {
        let _ =
            common::s2n_quic::provider::tls::rustls::rustls::crypto::aws_lc_rs::default_provider()
                .install_default();

        let tmp = TempDir::new()?;
        let certs_path = tmp.path().join("certs");
        std::fs::create_dir_all(&certs_path)?;
        let assets_path = tmp.path().join("assets");
        std::fs::create_dir_all(&assets_path)?;
        let db_path = tmp.path().join("loopback.sqlite3");
        std::fs::File::create(&db_path)?;

        let ca = GeneratedCa::generate(&["localhost".into(), "127.0.0.1".into()])?;
        std::fs::write(certs_path.join("ca.crt"), &ca.cert_pem)?;
        std::fs::write(certs_path.join("ca.key"), &ca.key_pem)?;
        let cert_service = CertificateService::new_shared(certs_path.to_str().unwrap())?;

        let mut options = ConnectOptions::new(format!("sqlite://{}", db_path.display()));
        options
            .max_connections(8)
            .min_connections(1)
            .connect_timeout(Duration::from_secs(3))
            .idle_timeout(Duration::from_secs(30))
            .sqlx_logging(false);
        let db = Arc::new(Database::connect(options).await?);
        Migrator::up(db.as_ref(), None).await?;

        // No certificate and no key: the listener under test has none, and the figment it
        // builds must not ask for one.
        let mut config = ApplicationConfig::default();
        config.database.scheme = "sqlite".into();
        config.database.database = db_path.to_string_lossy().into_owned();
        config.server.tls.certs_path = certs_path.to_string_lossy().into_owned();
        config.server.assets_path = assets_path.to_string_lossy().into_owned();
        config.server.mod_loopback = true;

        let (webhook_tx, mut webhook_rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move { while webhook_rx.recv().await.is_some() {} });
        let webhook = WebhookReceiver::new(webhook_tx);

        let playback = Arc::new(AudioPlaybackService::new(
            webhook.clone(),
            ".".to_string(),
            CancellationToken::new(),
            1,
        ));
        let bedrock = BedrockEventService::new_shared(
            playback.clone(),
            webhook.clone(),
            db.clone(),
            config.server.bedrock.proxy_event_freshness_threshold_secs,
        );
        let (metrics, _posthog) = MetricsService::new_shared(
            false,
            &config.server.tls.certs_path,
            "",
            Vec::new(),
            false,
            config.voice.recording.enabled,
            None,
        );
        let access_token_service =
            AccessTokenService::new_shared(db.clone(), Some(LOOPBACK_TOKEN.to_string()), true);

        let bind = ApiBind::reserve()?;
        let manager = RocketManager::new(
            config,
            RocketListener::ModLoopback,
            bind.clone(),
            webhook,
            CacheManager::new(),
            PlayerRegistrarService::new(db.clone(), cert_service.clone()),
            PlayerIdentityService::new(db.clone()),
            playback,
            bedrock,
            ChatService::new_shared(true),
            cert_service,
            CertificateRevocationService::new_shared(),
            None,
            metrics,
            bvc_server_lib::runtime::ReadinessState::new_shared(),
            CurrentNonce::new_shared(),
            None,
            access_token_service,
        );

        let task = tokio::spawn(async move {
            let _ = manager.start().await;
        });

        let server = Self {
            base_url: String::new(),
            _tmp: tmp,
            _task: task,
        };

        // `start` re-picks the port when the reservation was taken, so the address is read
        // after it settles rather than before.
        let probe = reqwest::Client::new();
        for _ in 0..100 {
            let base_url = format!("http://{}", bind.addr());
            if probe.get(format!("{base_url}/api/position")).send().await.is_ok() {
                return Ok(Self { base_url, ..server });
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Err(anyhow!("the loopback listener did not become ready"))
    }
}
