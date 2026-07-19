//! Builds and launches a Rocket server with only the routes the integration tests need.
//!
//! Production goes through `RocketManager::start()` which mounts everything including QUIC
//! state and audio playback. The tests only care about the admin + auth surface, so we
//! mount handlers directly. The TLS / mTLS figment matches production exactly via
//! `ApplicationConfig::get_rocket_config()`.

mod noop_inject_delivery;

use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};
use bvc_server_lib::config::ApplicationConfig;
use bvc_server_lib::http::pool::AppDb;
use bvc_server_lib::http::routes;
use bvc_server_lib::services::{CertificateService, PlayerIdentityService};
use common::ncryptflib as ncryptf;
use rocket::routes;
use sea_orm_rocket::Database;

use noop_inject_delivery::NoopInjectDelivery;

pub struct RocketHarness;

impl RocketHarness {
    pub async fn launch(
        config: ApplicationConfig,
        cert_service: Arc<CertificateService>,
        identity_service: PlayerIdentityService,
        mount_relay: bool,
    ) -> Result<tokio::task::JoinHandle<()>> {
        let figment = config
            .get_rocket_config()
            .map_err(|e| anyhow!("rocket figment: {}", e))?;

        let admin_routes = routes![
            routes::api::admin::user::create::create_user,
            routes::api::admin::user::banish::banish_user,
            routes::api::admin::user::generate_code::generate_code,
            routes::api::admin::permission::set::set_permission,
            routes::api::admin::permission::clear::clear_permission,
            routes::api::admin::permission::list::list_permissions,
        ];
        let auth_routes = routes![
            routes::api::auth::introspect::introspect,
            routes::api::auth::code::code_authenticate,
        ];
        let ncryptf_routes = routes![routes::ncryptf::ncryptf_ek_route,];

        let server_state = config.server.clone();
        let permissions = config.permissions.clone();
        let features = config.server.features.clone();
        let (metrics, _posthog) = bvc_server_lib::services::MetricsService::new_shared(
            false,
            &config.server.tls.certs_path,
            &config.server.tls.certificate,
        );

        let cache = cached::TimedCache::with_lifespan_and_refresh(
            std::time::Duration::from_secs(3600),
            true,
        );
        let cache = Arc::new(Mutex::new(cache));
        let cache_wrapper = ncryptf::rocket::CacheWrapper::TimedCache(cache);

        // Kept for the relay peering routes (offer / peer-redeem / peer-link), which
        // need an in-memory `ServerPeerStore`; `cert_service` itself is moved into
        // `.manage`.
        let cert_service_for_relay = cert_service.clone();
        let relay_certs_path = config.server.tls.certs_path.clone();

        // Control-plane state: a CacheManager (channel collection + state/pref caches)
        // and a WebhookReceiver whose queue is drained so `send_packet` fan-outs
        // succeed (the tests assert the HTTP surface, not the QUIC broadcast).
        let control_cache_manager = bvc_server_lib::stream::quic::CacheManager::new();
        let (webhook_tx, mut webhook_rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move { while webhook_rx.recv().await.is_some() {} });
        let control_webhook = bvc_server_lib::stream::quic::WebhookReceiver::new(webhook_tx);
        let control_routes = routes![
            routes::api::control::control,
            routes::api::state::get_state,
            routes::api::state::get_preferences,
        ];

        let mut rocket = rocket::custom(figment)
            .manage(control_cache_manager)
            .manage(control_webhook)
            .manage(identity_service)
            .manage(server_state)
            .manage(features)
            .manage(permissions)
            .manage(cert_service)
            .manage(cache_wrapper)
            .manage(metrics)
            .attach(AppDb::init())
            .mount("/ncryptf", ncryptf_routes)
            .mount("/api/admin", admin_routes)
            .mount("/api", auth_routes)
            .mount("/api", control_routes)
            .mount("/metrics", routes![routes::metrics::metrics]);

        if mount_relay {
            // Cross-server peering routes (offer / peer-redeem / peer-link). They
            // need an in-memory `ServerPeerStore`; discovery is decentralized via
            // the in-realm announce, so there is no registry/reachability to mount.
            let ca_pem = std::fs::read_to_string(format!("{}/ca.crt", relay_certs_path))
                .map_err(|e| anyhow!("read ca.crt for relay store: {}", e))?;
            let store =
                bvc_server_lib::relay::ServerPeerStore::new_shared(cert_service_for_relay, ca_pem);
            let inject: std::sync::Arc<dyn bvc_server_lib::relay::LocalInjectDelivery> =
                std::sync::Arc::new(NoopInjectDelivery);
            rocket = rocket.manage(store).manage(inject).mount(
                "/api/relay",
                routes![
                    routes::api::relay::offer::offer,
                    routes::api::relay::peer_link::peer_link,
                    routes::api::relay::peer_redeem::peer_redeem,
                ],
            );
        }

        let rocket = rocket.register(
            "/",
            rocket::catchers![rocket_governor::rocket_governor_catcher],
        );

        let handle = tokio::spawn(async move {
            let ignite = match rocket.ignite().await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("rocket ignite failed: {}", e);
                    return;
                }
            };
            if let Err(e) = ignite.launch().await {
                tracing::error!("rocket launch failed: {}", e);
            }
        });
        Ok(handle)
    }
}
