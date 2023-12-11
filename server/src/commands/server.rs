use clap::Parser;
use rcgen::{CertificateParams, Certificate, KeyPair, PKCS_ED25519, DistinguishedName, IsCa};
use rocket::time::OffsetDateTime;
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_subscriber::fmt::SubscriberBuilder;
use std::{process::exit, path::Path, fs::File, io::Write};
use crate::commands::State as StateConfig;
use crate::rs::routes;
use anyhow::anyhow;
use std::fs;
use faccess::PathExt;

use common::{
    ncryptflib as ncryptf,
    rocket::{self, routes},
    rocket_db_pools,
    ncryptflib::rocket::Fairing as NcryptfFairing,
    pool::redis::RedisDb,
};
use tracing::info;

use crate::config::ApplicationConfig;
use serde_json::Value;

/// Starts the BVC Server
#[derive(Debug, Parser, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Config {
    // Path to bvc configuration file
    #[clap(
        short,
        long,
        value_parser,
        required = false,
        default_value = "config.hcl"
    )]
    pub config_file: String,

    #[clap(skip)]
    pub config: ApplicationConfig,
}

impl Config {
    /// Starts Homemaker API server.
    pub async fn run<'a>(&'a self, cfg: &StateConfig) {
        match self.get_config_file() {
            Ok(hcl) => {
                let config = Config {
                    config_file: self.config_file.clone(),
                    config: hcl,
                };
                config.serve_internal(cfg).await;
            }
            Err(error) => {
                println!("{}", error);
                exit(1);
            }
        };
    }

    /// Internal serve function to operate with parsed configuration with config in place
    async fn serve_internal<'a>(&'a self, _cfg: &StateConfig) {
        // Setup and configure the application logger
        let out = &self.config.log.out;
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
        };

        subscriber
            .with_writer(non_blocking)
            .with_max_level(self.config.get_tracing_log_level())
            .with_level(true)
            .with_line_number(
                &self.config.log.level == "debug" || &self.config.log.level == "trace",
            )
            .with_file(&self.config.log.level == "debug" || &self.config.log.level == "trace")
            .compact()
            .init();

        info!("Logger established!");
        
        // Create the root CA certificate if it doesn't already exist.
        let root_ca_path_str = format!("{}/{}", &self.config.server.tls.certs_path, "ca.crt");
        let root_ca_key_path_str = format!("{}/{}", &self.config.server.tls.certs_path, "ca.key");
        let root_ca_path = Path::new(&root_ca_path_str);

        if !root_ca_path.exists() {
            let root_kp = match KeyPair::generate(&PKCS_ED25519) {
                Ok(r) => r,
                Err(_) => panic!("Unable to generate root key. Check the certs_path configuration variable to ensure the path is writable")
            };

            let mut distinguished_name = DistinguishedName::new();
            distinguished_name.push(rcgen::DnType::CommonName, "Bedrock Voice Chat");


            let mut cp = CertificateParams::new(vec![self.config.server.public_addr.clone()]);
            cp.alg = &PKCS_ED25519;
            cp.is_ca = IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
            cp.not_before = OffsetDateTime::now_utc();
            cp.distinguished_name = distinguished_name;
            cp.key_pair = Some(root_kp);

            let root_certificate = match Certificate::from_params(cp) {
                Ok(c) => c,
                Err(_) => panic!("Unable to generate root certificates. Check the certs_path configuration variable to ensure the path is writable")
            };

            let key = root_certificate.get_key_pair().serialize_pem();
            let cert = root_certificate.serialize_pem().unwrap();

            let mut key_file = File::create(root_ca_path_str).unwrap();
            key_file.write_all(cert.as_bytes()).unwrap();
            let mut cert_file = File::create(root_ca_key_path_str).unwrap();
            cert_file.write_all(key.as_bytes()).unwrap();
        }        

        // Launch Rocket and QUIC
        let mut tasks = Vec::new();

        // Launch Rocket for all web related tasks
        let app_config = self.config.clone().to_owned();
        let rocket_task = tokio::task::spawn(async move {
            ncryptf::ek_route!(RedisDb);
            match app_config.get_rocket_config() {
                Ok(figment) => {
                    let rocket = rocket::custom(figment)
                        .manage(app_config.server.clone())
                        .attach(RedisDb::init())
                        .attach(NcryptfFairing)
                        .mount("/api/auth", routes![routes::auth::authenticate])
                        //.mount(
                        //    "/ncryptf",
                        //    routes![
                        //        ncryptf_ek_route,
                        //        routes::ncryptf::token_info_route,
                        //        routes::ncryptf::token_revoke_route,
                        //        routes::ncryptf::token_refresh_route,
                        //    ],
                        //)
                        .mount("/api/mc", routes![routes::mc::position]);

                    if let Ok(ignite) = rocket.ignite().await {
                        info!("Rocket server is now running and awaiting request!");
                        let result = ignite.launch().await;
                        if result.is_err() {
                            println!("{}", result.unwrap_err());
                            exit(1);
                        }
                    }
                }
                Err(error) => {
                    println!("{}", error);
                    exit(1);
                }
            }
        });
        tasks.push(rocket_task);

        // Media over QUIC IETF (MoQ) server
        // The relay connects publishers back to subscribers (which should be a 1:1 match)
        let moq_relay_config = self.config.clone().to_owned();
        let moq_relay_task = tokio::task::spawn(async move {
            // @TODO: Implement MoQ Relay Transport
            drop(moq_relay_config);
        });

        tasks.push(moq_relay_task);
        for task in tasks {
            #[allow(unused_must_use)]
            {
                task.await;
            }
        }
    }

    /// Reads in the HCL configuration file
    fn get_config_file<'a>(&'a self) -> std::result::Result<ApplicationConfig, anyhow::Error> {
        if let Ok(config) = fs::read_to_string(&self.config_file) {
            if let Ok(hcl) = hcl::from_str::<Value>(&config.as_str()) {
                let app_config: Result<ApplicationConfig, serde_json::Error> =
                    serde_json::from_value(hcl);
                if app_config.is_ok() {
                    let acr = app_config.unwrap();
                    return Ok::<ApplicationConfig, anyhow::Error>(acr);
                } else {
                    return Err(anyhow!(app_config.unwrap_err()));
                }
            }
        }

        return Err(anyhow!("Unable to read or parse configuration file."));
    }
}