pub const VERSION: &str = env!("CARGO_PKG_VERSION");

extern crate common;

#[macro_use]
extern crate rocket;

pub(crate) mod config;
pub mod http;
pub mod services;
pub(crate) mod stream;

pub mod ffi;
pub mod runtime;

pub use config::ApplicationConfig;
pub use runtime::{RuntimeState, ServerRuntime};

/// Build a minimal Rocket instance suitable for integration tests.
///
/// Mounts only the `/api/auth/minecraft` route and wires the state it needs.
/// Gated behind `cfg(feature = "test-utils")` so it is never compiled into
/// production binaries.
#[cfg(feature = "test-utils")]
pub async fn build_test_rocket(
    authenticator: std::sync::Arc<dyn common::auth::MinecraftAuthenticator>,
    db_url: String,
) -> rocket::Rocket<rocket::Build> {
    use std::sync::Arc;
    use rocket::figment::Figment;
    use sea_orm_rocket::Database as _;
    use common::ncryptflib::randombytes_buf;

    let cert_service = Arc::new(
        services::CertificateService::new_for_test()
            .expect("test CA generation"),
    );

    let certs_dir = std::env::temp_dir().join(format!("bvc_test_certs_{}", std::process::id()));
    std::fs::create_dir_all(&certs_dir).expect("create temp certs dir");
    std::fs::write(certs_dir.join("ca.crt"), cert_service.ca_cert_pem())
        .expect("write test CA cert");

    let mut server_config = config::Server::default();
    server_config.tls.certs_path = certs_dir.to_string_lossy().into_owned();

    let permissions = config::Permissions::default();
    let identity_db = Arc::new(
        sea_orm::Database::connect(&db_url)
            .await
            .expect("identity service db"),
    );
    let identity_service = services::PlayerIdentityService::new(identity_db.clone());
    let player_registrar = services::PlayerRegistrarService::new(identity_db, cert_service.clone());

    let (webhook_tx, _webhook_rx) = tokio::sync::mpsc::unbounded_channel();
    let webhook_receiver = stream::quic::WebhookReceiver::new(webhook_tx);

    let figment = Figment::from(rocket::Config::default())
        .merge(("log_level", rocket::config::LogLevel::Off))
        .merge(("secret_key", randombytes_buf(32)))
        .merge((
            "databases.app",
            sea_orm_rocket::Config {
                url: db_url,
                min_connections: None,
                max_connections: 4,
                connect_timeout: 3,
                idle_timeout: None,
                sqlx_logging: false,
            },
        ));

    rocket::custom(figment)
        .manage(server_config)
        .manage(permissions)
        .manage(cert_service)
        .manage(identity_service)
        .manage(player_registrar)
        .manage(webhook_receiver)
        .manage(authenticator)
        .attach(http::pool::AppDb::init())
        .mount(
            "/api",
            rocket::routes![
                http::routes::api::auth::minecraft::authenticate,
                http::routes::api::auth::link_java::link_java_identity,
                http::routes::api::positions::update_position,
            ],
        )
}

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub struct BvcServer;

impl BvcServer {
    pub fn new(config: ApplicationConfig) -> Result<ServerRuntime, anyhow::Error> {
        Self::init_platform();
        ServerRuntime::new(config)
    }

    fn init_platform() {
        let _ = common::s2n_quic::provider::tls::rustls::rustls::crypto::aws_lc_rs::default_provider()
            .install_default();

        #[cfg(target_os = "windows")]
        {
            windows_targets::link!("winmm.dll" "system" fn timeBeginPeriod(uperiod: u32) -> u32);
            windows_targets::link!("ntdll.dll" "system" fn NtQueryTimerResolution(
                minimumresolution: *mut u32,
                maximumresolution: *mut u32,
                currentresolution: *mut u32,
            ) -> i32);

            unsafe {
                let mut min_res = 0u32;
                let mut max_res = 0u32;
                let mut current_res = 0u32;
                NtQueryTimerResolution(&mut min_res, &mut max_res, &mut current_res);
                let current_ms = current_res as f64 / 10_000.0;
                tracing::info!("Current Windows timer resolution: {:.2}ms", current_ms);

                timeBeginPeriod(1);

                NtQueryTimerResolution(&mut min_res, &mut max_res, &mut current_res);
                let new_ms = current_res as f64 / 10_000.0;
                tracing::info!("Set Windows timer resolution to 1ms (actual: {:.2}ms)", new_ms);

                if new_ms > 2.0 {
                    tracing::warn!("Timer resolution is degraded ({:.2}ms). This may cause audio jitter!", new_ms);
                }
            }
        }
    }
}
