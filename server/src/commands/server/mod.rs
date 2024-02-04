use crate::commands::Config as StateConfig;

use clap::Parser;
use common::structs::packet::QuicNetworkPacket;
use rcgen::{
    Certificate,
    CertificateParams,
    DistinguishedName,
    IsCa,
    KeyPair,
    PKCS_ED25519,
    SanType,
    ExtendedKeyUsagePurpose,
    KeyUsagePurpose,
};
use rocket::time::OffsetDateTime;
use rocket::time::Duration;
use anyhow::anyhow;

use faccess::PathExt;

use std::{ fs::File, io::Write, path::Path, process::exit };
use std::sync::Arc;
use tracing_appender::non_blocking::{ NonBlocking, WorkerGuard };
use tracing_subscriber::fmt::SubscriberBuilder;
use tracing::info;
use common::structs::channel::Channel;
mod web;
mod quic;

const DEADQUEUE_SIZE: usize = 10_000;
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

        subscriber
            .with_writer(non_blocking)
            .with_max_level(cfg.config.get_tracing_log_level())
            .with_level(true)
            .with_line_number(&cfg.config.log.level == "debug" || &cfg.config.log.level == "trace")
            .with_file(&cfg.config.log.level == "debug" || &cfg.config.log.level == "trace")
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
        let channel_cache = Arc::new(
            async_mutex::Mutex::new(
                moka::future::Cache::<String, common::structs::channel::Channel>
                    ::builder()
                    .max_capacity(100)
                    .build()
            )
        );

        // The deadqueue is our primary way of sending messages to the QUIC stream to broadcast to clients
        // without needing to setup a new stream.
        // The QUIC server polls this queue and is configured to handle inbound packets and advertise them
        // To the appropriate clients
        let queue = Arc::new(deadqueue::limited::Queue::<QuicNetworkPacket>::new(DEADQUEUE_SIZE));

        let mut tasks = Vec::new();

        // Task for Rocket https API
        // The API handles player positioning data from the game/source, and various non-streaming events
        // such as channel creation, or anything else that may require a "broadcast".
        let rocket_task = web::get_task(&cfg.config.clone(), queue.clone(), channel_cache.clone());
        tasks.push(rocket_task);

        // Tasks for the QUIC streaming server
        // The QUIC server handles broadcasting of messages and raw packets to clients
        // This includes audio frame, player positioning data, and more.
        match quic::get_task(&cfg.config.clone(), queue.clone(), channel_cache.clone()).await {
            Ok(t) => {
                for task in t {
                    tasks.push(task);
                }
            }
            Err(e) => {
                panic!("Something went wrong setting up the QUIC server {}", e.to_string());
            }
        }

        for task in tasks {
            #[allow(unused_must_use)]
            {
                task.await;
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
                    return Err(
                        anyhow!("Could not create directory {}", cert_root_path.to_string_lossy())
                    );
                }
            }
        }

        // Create the root CA certificate if it doesn't already exist.

        let root_kp = match KeyPair::generate(&PKCS_ED25519) {
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

        let mut cp = CertificateParams::new(vec!["127.0.0.1".to_string(), "localhost".to_string()]);

        cp.subject_alt_names = vec![
            SanType::DnsName(String::from("localhost")),
            SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))),
            SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0))),
            SanType::IpAddress(
                std::net::IpAddr::V6(std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1))
            )
        ];
        cp.alg = &PKCS_ED25519;
        cp.is_ca = IsCa::NoCa;
        cp.not_before = OffsetDateTime::now_utc().checked_sub(Duration::days(3)).unwrap();
        cp.distinguished_name = distinguished_name;
        cp.key_pair = Some(root_kp);
        cp.use_authority_key_identifier_extension = true;

        cp.key_usages = vec![KeyUsagePurpose::KeyCertSign];
        cp.extended_key_usages = vec![
            ExtendedKeyUsagePurpose::ClientAuth,
            ExtendedKeyUsagePurpose::ServerAuth
        ];
        let root_certificate = match Certificate::from_params(cp) {
            Ok(c) => c,
            Err(_) =>
                panic!(
                    "Unable to generate root certificates. Check the certs_path configuration variable to ensure the path is writable"
                ),
        };

        let cert = root_certificate.serialize_pem_with_signer(&root_certificate).unwrap();
        let key = root_certificate.get_key_pair().serialize_pem();

        let mut key_file = File::create(root_ca_path_str).unwrap();
        key_file.write_all(cert.as_bytes()).unwrap();
        let mut cert_file = File::create(root_ca_key_path_str).unwrap();
        cert_file.write_all(key.as_bytes()).unwrap();

        Ok((cert, key))
    }
}
