//! Builds and launches a Rocket server with only the routes the integration tests need.
//!
//! Production goes through `RocketManager::start()` which mounts everything including QUIC
//! state and audio playback. The tests only care about the admin + auth surface, so we
//! mount handlers directly. The TLS / mTLS figment matches production exactly via
//! `ApplicationConfig::get_rocket_config()`.

use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use bvc_server_lib::config::ApplicationConfig;
use bvc_server_lib::http::pool::AppDb;
use bvc_server_lib::http::routes;
use bvc_server_lib::services::CertificateService;
use bvc_server_lib::services::relay::EndpointReachability;
use common::ncryptflib as ncryptf;
use common::structs::relay::RelayEndpoint;
use rocket::routes;
use sea_orm_rocket::Database;

// Permissive endpoint-control checker for the discovery-plane integration tests:
// the real `HttpEndpointReachability` would try to HTTPS-fetch the
// fake `*.example.com` endpoints. This stub stands in for "the endpoint served
// the relay's nonce back", so the tests still exercise the token-gated register
// + scoped lookup mechanics. The deny path is unit-tested in `registry.rs`.
struct AlwaysReachable;

#[async_trait::async_trait]
impl EndpointReachability for AlwaysReachable {
    async fn serves_nonce(&self, _endpoint: &RelayEndpoint, _nonce: &str) -> bool {
        true
    }
}

pub struct RocketHarness;

impl RocketHarness {
    pub async fn launch(
        config: ApplicationConfig,
        cert_service: Arc<CertificateService>,
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
        let ncryptf_routes = routes![
            routes::ncryptf::ncryptf_ek_route,
        ];

        let server_state = config.server.clone();
        let permissions = config.permissions.clone();
        let features = config.server.features.clone();

        let cache = cached::TimedCache::with_lifespan_and_refresh(
            std::time::Duration::from_secs(3600),
            true,
        );
        let cache = Arc::new(Mutex::new(cache));
        let cache_wrapper = ncryptf::rocket::CacheWrapper::TimedCache(cache);

        let relay_enabled = features.relay.enabled;

        let mut rocket = rocket::custom(figment)
            .manage(server_state)
            .manage(features)
            .manage(permissions)
            .manage(cert_service)
            .manage(cache_wrapper)
            .attach(AppDb::init())
            .mount("/ncryptf", ncryptf_routes)
            .mount("/api/admin", admin_routes)
            .mount("/api", auth_routes);

        if relay_enabled {
            let reachability: Arc<dyn EndpointReachability> = Arc::new(AlwaysReachable);
            rocket = rocket
                .manage(bvc_server_lib::services::RelayRegistry::new_shared())
                .manage(reachability)
                .mount(
                    "/relay",
                    routes![
                        routes::relay::challenge::challenge,
                        routes::relay::register::register,
                        routes::relay::lookup::lookup,
                    ],
                );
        }

        let rocket = rocket;

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
