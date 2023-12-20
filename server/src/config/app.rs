use common::rocket::{
    data::{Limits, ToByteUnit},
    figment::Figment
};

use serde::{Deserialize, Serialize};
use tracing::Level;
use anyhow::anyhow;

/// Application Configuration as described in homemaker.hcl configuration file
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfig {
    pub redis: ApplicationConfigRedis,
    pub server: ApplicationConfigServer,
    pub log: ApplicationConfigLogger,
}

/// Database configuration for Redis
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfigRedis {
    #[serde(default)]
    pub database: String,
    pub host: String,
    #[serde(default)]
    pub port: u32,
}

/// Common server configuration
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfigServer {
    #[serde(default)]
    pub listen: String,
    #[serde(default)]
    pub port: u32,
    #[serde(default)]
    pub public_addr: String,
    pub tls: ApplicationConfigServerTLS,
    pub minecraft: ApplicationConfigMinecraft
}

/// TLS Configuration for server
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfigServerTLS {
    certificate: String,
    key: String,
    #[serde(default)]
    pub so_reuse_port: bool,
    pub certs_path: String
}

/// Minecraft Configuration
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfigMinecraft {
    pub access_token: String,
    pub client_id: String,
    pub client_secret: String
}

/// OAuth2 Client Configuration
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfigLogger {
    pub level: String,
    pub out: String,
}

/// Default values for ApplicationConfig struct
impl Default for ApplicationConfig {
    fn default() -> Self {
        ApplicationConfig {
            redis: ApplicationConfigRedis {
                database: String::from(""),
                host: String::from("127.0.0.1"),
                port: 6379,
            },
            server: ApplicationConfigServer {
                listen: String::from("127.0.0.1"),
                port: 443,
                public_addr: String::from("127.0.0.1"),
                tls: ApplicationConfigServerTLS {
                    certificate: String::from("/etc/bvc/server.crt"),
                    key: String::from("/etc/bvc/server.key"),
                    so_reuse_port: false,
                    certs_path: String::from("/etc/bvc/certificates")
                },
                minecraft: ApplicationConfigMinecraft {
                    access_token: String::from(""),
                    client_id: String::from(""),
                    client_secret: String::from("")
                }
            },
            log: ApplicationConfigLogger {
                level: String::from("info"),
                out: String::from("stdout"),
            },
        }
    }
}

impl ApplicationConfig {
    /// Returns the appropriate log level for Rocket.rs
    pub fn get_rocket_log_level<'a>(&'a self) -> common::rocket::config::LogLevel {
        match self.log.level.as_str() {
            "info" => common::rocket::config::LogLevel::Normal,
            "trace" => common::rocket::config::LogLevel::Debug,
            "debug" => common::rocket::config::LogLevel::Normal,
            "error" => common::rocket::config::LogLevel::Critical,
            "warn" => common::rocket::config::LogLevel::Critical,
            _ => common::rocket::config::LogLevel::Off,
        }
    }

    /// Returns the appropriate log level for tokio/tracing
    pub fn get_tracing_log_level<'a>(&'a self) -> tracing::Level {
        match self.log.level.as_str() {
            "info" => Level::INFO,
            "trace" => Level::TRACE,
            "debug" => Level::DEBUG,
            "warn" => Level::WARN,
            _ => Level::ERROR,
        }
    }

    pub fn get_rocket_config<'a>(&'a self) -> Result<Figment, anyhow::Error> {
        if !std::path::Path::new(&self.server.tls.certificate).exists()
            || !std::path::Path::new(&self.server.tls.key).exists()
        {
            return Err(anyhow!("TLS certificate or private key is not valid"));
        }

        let figment = common::rocket::Config::figment()
            .merge(("profile", common::rocket::figment::Profile::new("release")))
            .merge(("ident", false))
            .merge(("log_level", self.get_rocket_log_level()))
            .merge(("port", &self.server.port))
            .merge(("address", &self.server.listen))
            .merge(("limits", Limits::new().limit("json", 10.megabytes())))
            .merge(("tls.certs", &self.server.tls.certificate))
            .merge(("tls.key", &self.server.tls.key))
            .merge(("minecraft.access_token", &self.server.minecraft.access_token))
            .merge((
                "databases.cache",
                common::rocket_db_pools::Config {
                    url: format!(
                        "redis://{}:{}/{}",
                        self.redis.host, self.redis.port, self.redis.database
                    ),
                    min_connections: None,
                    max_connections: 1024,
                    connect_timeout: 3,
                    idle_timeout: None,
                },
            ));

        return Ok(figment);
    }
}