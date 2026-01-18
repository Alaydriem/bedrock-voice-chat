mod database;
mod logger;
mod minecraft;
mod server;
mod tls;
mod voice;

pub use database::ApplicationConfigDatabase;
pub use logger::ApplicationConfigLogger;
pub use minecraft::ApplicationConfigMinecraft;
pub use server::ApplicationConfigServer;
pub use tls::ApplicationConfigServerTLS;
pub use voice::ApplicationConfigVoice;

use common::ncryptflib::randombytes_buf;
use rocket::{
    data::{Limits, ToByteUnit},
    figment::Figment,
};

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use tracing::Level;

/// Application Configuration as described in homemaker.hcl configuration file
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfig {
    #[serde(default)]
    pub database: ApplicationConfigDatabase,
    pub server: ApplicationConfigServer,
    #[serde(default)]
    pub log: ApplicationConfigLogger,
    #[serde(default)]
    pub voice: ApplicationConfigVoice,
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        ApplicationConfig {
            database: ApplicationConfigDatabase::default(),
            server: ApplicationConfigServer::default(),
            voice: ApplicationConfigVoice::default(),
            log: ApplicationConfigLogger::default(),
        }
    }
}

impl ApplicationConfig {
    fn get_dsn(&self) -> String {
        match self.database.scheme.as_str() {
            "sqlite" | "sqlite3" => {
                let path = std::path::Path::new(&self.database.database);
                if !path.exists() {
                    match std::fs::File::create(&self.database.database) {
                        Ok(_) => {}
                        Err(_e) => {
                            panic!(
                                "Verify that {} exists and is writable. You may need to create this file.",
                                &self.database.database
                            );
                        }
                    }
                }

                format!("sqlite://{}", &self.database.database)
            }
            "mysql" => format!(
                "mysql://{}:{}@{}:{}/{}",
                &self.database.username.clone().unwrap_or(String::from("")),
                &self.database.password.clone().unwrap_or(String::from("")),
                &self
                    .database
                    .host
                    .clone()
                    .unwrap_or(String::from("127.0.0.1")),
                &self.database.port.unwrap_or(3306),
                &self.database.database
            ),
            _ => format!("sqlite://{}", "/etc/bvc/bvc.sqlite3"),
        }
    }

    /// Returns the appropriate log level for Rocket.rs
    pub fn get_rocket_log_level(&self) -> rocket::config::LogLevel {
        match self.log.level.as_str() {
            "trace" => rocket::config::LogLevel::Debug,
            "debug" => rocket::config::LogLevel::Normal,
            "info" => rocket::config::LogLevel::Critical,
            "error" => rocket::config::LogLevel::Critical,
            "warn" => rocket::config::LogLevel::Critical,
            _ => rocket::config::LogLevel::Off,
        }
    }

    /// Returns the appropriate log level for tokio/tracing
    pub fn get_tracing_log_level(&self) -> tracing::Level {
        match self.log.level.as_str() {
            "info" => Level::INFO,
            "trace" => Level::TRACE,
            "debug" => Level::DEBUG,
            "warn" => Level::WARN,
            _ => Level::ERROR,
        }
    }

    pub fn get_rocket_config(&self) -> Result<Figment, anyhow::Error> {
        if !std::path::Path::new(&self.server.tls.certificate).exists()
            || !std::path::Path::new(&self.server.tls.key).exists()
        {
            return Err(anyhow!("TLS certificate or private key is not valid"));
        }

        tracing::info!("Database: {}", self.get_dsn().to_string());
        let figment = rocket::Config::figment()
            .merge(("profile", rocket::figment::Profile::new("release")))
            .merge(("ident", false))
            .merge(("log_level", self.get_rocket_log_level()))
            .merge(("port", &self.server.port))
            .merge(("address", &self.server.listen))
            .merge(("limits", Limits::new().limit("json", (10).megabytes())))
            .merge(("secret_key", randombytes_buf(32)))
            .merge(("tls.certs", &self.server.tls.certificate))
            .merge(("tls.key", &self.server.tls.key))
            .merge((
                "tls.mutual.ca_certs",
                format!("{}/ca.crt", &self.server.tls.certs_path),
            ))
            .merge(("tls.mutual.mandatory", false))
            .merge((
                "minecraft.access_token",
                &self.server.minecraft.access_token,
            ))
            .merge((
                "databases.app",
                sea_orm_rocket::Config {
                    url: self.get_dsn().to_string(),
                    min_connections: None,
                    max_connections: 1024,
                    connect_timeout: 3,
                    idle_timeout: Some(1),
                    sqlx_logging: false,
                },
            ));

        Ok(figment)
    }
}
