//! Builds and launches a Rocket server with only the routes the integration tests need.
//!
//! Production goes through `RocketManager::start()` which mounts everything including QUIC
//! state and audio playback. The tests only care about the admin + auth surface, so we
//! mount handlers directly. The TLS / mTLS figment matches production exactly via
//! `ApplicationConfig::get_rocket_config()`.

use std::sync::Arc;

use anyhow::{anyhow, Result};
use bvc_server_lib::config::ApplicationConfig;
use bvc_server_lib::http::pool::AppDb;
use bvc_server_lib::http::routes;
use bvc_server_lib::services::CertificateService;
use rocket::routes;
use sea_orm_rocket::Database;

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
            routes::api::auth::code_json::code_authenticate_json,
        ];

        let server_state = config.server.clone();
        let permissions = config.permissions.clone();
        let features = config.server.features.clone();

        let rocket = rocket::custom(figment)
            .manage(server_state)
            .manage(features)
            .manage(permissions)
            .manage(cert_service)
            .attach(AppDb::init())
            .mount("/api/admin", admin_routes)
            .mount("/api", auth_routes);

        let handle = tokio::spawn(async move {
            let ignite = match rocket.ignite().await {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("rocket ignite failed: {}", e);
                    return;
                }
            };
            if let Err(e) = ignite.launch().await {
                eprintln!("rocket launch failed: {}", e);
            }
        });
        Ok(handle)
    }
}
