//! Builds and launches a Rocket server with only the routes the integration tests need.
//!
//! Production goes through `RocketManager::start()` which mounts everything including QUIC
//! state and audio playback. The tests only care about the admin + auth surface, so we
//! mount handlers directly. The TLS / mTLS figment matches production exactly via
//! `ApplicationConfig::get_rocket_config()`.


use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};
use bvc_server_lib::config::ApplicationConfig;
use bvc_server_lib::http::pool::AppDb;
use bvc_server_lib::http::routes;
use bvc_server_lib::services::{
    CertificateRevocationService, CertificateService, PlayerIdentityService,
};
use common::ncryptflib as ncryptf;
use rocket::routes;
use sea_orm_rocket::Database;


pub struct RocketHarness;

impl RocketHarness {
    pub async fn launch(
        config: ApplicationConfig,
        cert_service: Arc<CertificateService>,
        identity_service: PlayerIdentityService,
        readiness: Arc<bvc_server_lib::runtime::ReadinessState>,
        // Passed in rather than built here so the test holds the same instance Rocket does.
        // Two instances would each carry their own cache, and a revocation written by a test
        // would never be seen by the running server.
        revocations: Arc<CertificateRevocationService>,
        // Held by the test as well as by Rocket, so a test can set the nonce the relay
        // would have pushed over the enrollment session and then read it back.
        nonce: Arc<bvc_server_lib::services::CurrentNonce>,
        // The credential source for the `GameAccessToken` guard. Every game route fails at
        // that guard if Rocket is not managing it.
        access_token_service: Arc<bvc_server_lib::services::AccessTokenService>,
        #[cfg(feature = "bedrock")]
        transfer_cache: bvc_server_lib::services::bedrock::TransferTargetCache,
    ) -> Result<tokio::task::JoinHandle<()>> {
        // Production puts the demultiplexer on the public port and Rocket on loopback.
        // The harness dials Rocket directly, so `server.port` is where it must bind.
        let bind = std::net::SocketAddr::from((
            std::net::Ipv4Addr::LOCALHOST,
            u16::try_from(config.server.port).map_err(|_| anyhow!("port out of range"))?,
        ));
        let figment = config
            .get_rocket_config(bind)
            .map_err(|e| anyhow!("rocket figment: {}", e))?;
        let cert_path_for_health = config.server.tls.certificate.clone();

        let admin_routes = routes![
            routes::api::admin::user::create::create_user,
            routes::api::admin::user::list::list_users,
            routes::api::admin::user::banish::banish_user,
            routes::api::admin::user::generate_code::generate_code,
            routes::api::admin::permission::set::set_permission,
            routes::api::admin::permission::clear::clear_permission,
            routes::api::admin::permission::list::list_permissions,
            routes::api::admin::relay::peerlink::relay_peerlink,
            routes::api::admin::relay::worlds::relay_worlds,
            routes::api::admin::token::list::list_tokens,
            routes::api::admin::token::mint::mint_token,
            routes::api::admin::token::revoke::revoke_token,
            routes::api::admin::token::rotate::rotate_token,
            routes::api::admin::token::legacy::legacy_token,
        ];
        let auth_routes = routes![
            routes::api::auth::introspect::introspect,
            routes::api::auth::code::code_authenticate,
        ];
        let ncryptf_routes = routes![routes::ncryptf::ncryptf_ek_route,];

        let server_state = config.server.clone();
        let permissions = config.permissions.clone();
        let features = config.server.features.clone();
        let voice = config.voice.clone();
        let (metrics, _posthog) = bvc_server_lib::services::MetricsService::new_shared(
            false,
            &config.server.tls.certs_path,
            &config.server.tls.certificate,
            Vec::new(),
            false,
            config.voice.recording.enabled,
            None,
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
            routes::api::clients::live_clients,
        ];

        // Player-authenticated routes. These are what exercise `PlayerGuard`, and the channel
        // event dispositions, so they are mounted even though the admin/auth surface does not
        // need them.
        let channel_routes = routes![
            routes::api::channel::create::channel_create,
            routes::api::channel::channel_list,
            routes::api::channel::delete::channel_delete,
            routes::api::channel::event::channel_event,
            routes::api::channel::rename::channel_rename,
        ];

        #[cfg(feature = "bedrock")]
        let bedrock_routes = routes![routes::api::bedrock::transfer::register_transfer_target];

        let mut rocket = rocket::custom(figment)
            .manage(bvc_server_lib::services::HealthService::new_shared(
                readiness,
                cert_path_for_health,
            ))
            .manage(control_cache_manager)
            .manage(control_webhook)
            .manage(identity_service)
            .manage(server_state)
            .manage(features)
            .manage(voice)
            .manage(permissions)
            .manage(cert_service)
            .manage(revocations)
            .manage(cache_wrapper)
            .manage(metrics)
            // No `peer` block, so no peer endpoint — which is the state every
            // server is in by default, and the one the peer link route reports
            // as a 404.
            .manage(nonce)
            .manage(access_token_service)
            .manage(None::<std::sync::Arc<bvc_server_lib::relay::PeerPlane>>)
            .attach(AppDb::init())
            .mount("/ncryptf", ncryptf_routes)
            .mount("/api/admin", admin_routes)
            .mount("/api", auth_routes)
            .mount("/api", control_routes)
            .mount("/api/channel", channel_routes)
            .mount("/api", routes![routes::api::server_config::get_config])
            .mount("/metrics", routes![routes::metrics::metrics])
            .mount(
                "/health",
                routes![
                    routes::api::health::enrollment_nonce::enrollment_nonce,
                    routes::api::health::liveness::liveness,
                    routes::api::health::readiness::readiness,
                ],
            );

        #[cfg(feature = "bedrock")]
        {
            rocket = rocket.manage(transfer_cache).mount("/api", bedrock_routes);
        }

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
