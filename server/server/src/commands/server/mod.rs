use crate::commands::Config as StateConfig;

use anyhow::anyhow;
use clap::Parser;
use rcgen::{
    CertificateParams, DistinguishedName, ExtendedKeyUsagePurpose, IsCa, KeyPair, KeyUsagePurpose,
};
use rocket::time::Duration;
use rocket::time::OffsetDateTime;

use faccess::PathExt;

use std::sync::Arc;
use std::{fs::File, io::Write, path::Path, process::exit};
use tracing::info;
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_subscriber::fmt::SubscriberBuilder;

/// Starts the BVC Server
#[derive(Debug, Parser, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Config {}

impl Config {
    /// Starts Homemaker API server.
    pub async fn run<'a>(&'a self, cfg: &StateConfig) {
        // Setup and configure the application logger
        let out = &cfg.config.log.out;
        let subscriber: SubscriberBuilder = tracing_subscriber::fmt();
        let non_blocking: NonBlocking;
        let _guard: WorkerGuard;
        match out.to_lowercase().as_str() {
            "stdout" => {
                (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());
            }
            _ => {
                let path = Path::new(out);
                if !path.exists() || !path.writable() {
                    println!("{} doesn't exist or is not writable", out);
                    exit(1);
                }
                let file_appender = tracing_appender::rolling::daily(out, "homemaker.log");
                (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
            }
        }

        let env_filter = match cfg.config.get_tracing_log_level() {
            tracing::Level::INFO => "info,hyper=off,rustls=off,rocket::server=off",
            tracing::Level::DEBUG => "info",
            tracing::Level::TRACE => "debug",
            tracing::Level::ERROR => "error,hyper=off,rustls=off,rocket::server=off",
            tracing::Level::WARN => "warn,hyper=off,rustls=off,rocket::server=off",
        };

        subscriber
            .with_writer(non_blocking)
            .with_max_level(cfg.config.get_tracing_log_level())
            .with_level(true)
            .with_line_number(&cfg.config.log.level == "trace")
            .with_file(&cfg.config.log.level == "trace")
            .with_env_filter(env_filter)
            .compact()
            .init();

        info!("Logger established!");

        match self.generate_ca(&cfg).await {
            Ok(_) => {}
            Err(e) => {
                println!("{}", e);
                exit(1);
            }
        }

        // State cache for recording groups a player is in.
        let channel_cache = Arc::new(async_mutex::Mutex::new(
            moka::future::Cache::<String, common::structs::channel::Channel>::builder()
                .max_capacity(100)
                .build(),
        ));

        // QUIC server manager is our main entry point - it should run until shutdown
        let mut quic_manager = crate::stream::quic::QuicServerManager::new(cfg.config.clone());
        let webhook_receiver = quic_manager.get_webhook_receiver().clone();
        let cache_manager = quic_manager.get_cache_manager();

        // Create Rocket manager
        let rocket_manager = crate::rs::manager::RocketManager::new(
            cfg.config.clone(),
            webhook_receiver,
            channel_cache.clone(),
            cache_manager,
        );

        // QUIC manager start() should be our main blocking entry point
        // It runs until SIGTERM and manages all QUIC connections, including Rocket
        tokio::select! {
            result = quic_manager.start() => {
                match result {
                    Ok(_) => tracing::info!("QUIC server stopped normally"),
                    Err(e) => tracing::error!("QUIC server error: {}", e),
                }
            }
            result = rocket_manager.start() => {
                match result {
                    Ok(_) => tracing::info!("Rocket server stopped normally"),
                    Err(e) => tracing::error!("Rocket server error: {}", e),
                }
            }
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Received SIGTERM, shutting down...");
                if let Err(e) = quic_manager.stop().await {
                    tracing::error!("Error during shutdown: {}", e);
                }
            }
        }
    }

    /// Generates the root CA
    async fn generate_ca(&self, config: &StateConfig) -> Result<(String, String), anyhow::Error> {
        let certs_path = &config.config.server.tls.certs_path;
        let root_ca_path_str = format!("{}/{}", &certs_path, "ca.crt");
        let root_ca_key_path_str = format!("{}/{}", &certs_path, "ca.key");

        // If the certificates already exist, just return them
        if Path::new(&root_ca_key_path_str).exists() {
            return Ok((
                std::fs::read_to_string(root_ca_path_str).unwrap(),
                std::fs::read_to_string(root_ca_key_path_str).unwrap(),
            ));
        }

        let cert_root_path = Path::new(&certs_path);
        if !cert_root_path.exists() {
            match std::fs::create_dir_all(cert_root_path) {
                Ok(_) => {}
                Err(_) => {
                    return Err(anyhow!(
                        "Could not create directory {}",
                        cert_root_path.to_string_lossy()
                    ));
                }
            }
        }

        // Create the root CA certificate if it doesn't already exist.
        let root_kp = match KeyPair::generate() {
            Ok(r) => r,
            Err(_) => {
                return Err(
                    anyhow!(
                        "Unable to generate root key. Check the certs_path configuration variable to ensure the path is writable"
                    )
                );
            }
        };

        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(rcgen::DnType::CommonName, "Bedrock Voice Chat");

        let mut san_names = config.config.server.tls.names.clone();
        san_names.append(&mut config.config.server.tls.ips.clone());
        let root_certificate = match CertificateParams::new(san_names) {
            Ok(mut ca_params) => {
                ca_params.is_ca = IsCa::NoCa;
                ca_params.not_before = OffsetDateTime::now_utc()
                    .checked_sub(Duration::days(3))
                    .unwrap();
                ca_params.distinguished_name = distinguished_name;
                ca_params.use_authority_key_identifier_extension = true;

                ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign];
                ca_params.extended_key_usages = vec![
                    ExtendedKeyUsagePurpose::ClientAuth,
                    ExtendedKeyUsagePurpose::ServerAuth,
                ];

                ca_params.self_signed(&root_kp)?
            }
            Err(_) => {
                panic!(
                    "Unable to generate root certificates. Check the certs_path configuration variable to ensure the path is writable"
                )
            }
        };

        let cert = root_certificate.pem();
        let key = root_kp.serialize_pem();

        let mut key_file = File::create(root_ca_path_str).unwrap();
        key_file.write_all(cert.as_bytes()).unwrap();
        let mut cert_file = File::create(root_ca_key_path_str).unwrap();
        cert_file.write_all(key.as_bytes()).unwrap();

        Ok((cert, key))
    }
}
