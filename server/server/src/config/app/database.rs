use anyhow::anyhow;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use serde::{Deserialize, Serialize};

fn default_db_scheme() -> String {
    "sqlite3".to_string()
}

fn default_db_database() -> String {
    "./bvc.sqlite3".to_string()
}

// Everything outside RFC 3986 unreserved characters is percent-encoded when
// embedded in a DSN, so credentials, database names, and file paths cannot
// corrupt the URL or inject query parameters.
const DSN_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// Database configuration for PostgreSQL/MySQL/MariaDB/SQLite
#[derive(Serialize, Deserialize, Debug, Clone, schemars::JsonSchema)]
pub struct Database {
    #[serde(default = "default_db_scheme")]
    pub scheme: String,
    #[serde(default = "default_db_database")]
    pub database: String,
    pub host: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    #[serde(default)]
    pub port: Option<u32>,
    // disable | prefer | require | verify-ca | verify-full
    // (translated to the sqlx-mysql vocabulary for the mysql scheme)
    #[serde(default)]
    pub ssl_mode: Option<String>,
    // Path to the CA certificate used to verify the database server
    #[serde(default)]
    pub ssl_root_cert: Option<String>,
    // Path to the client certificate presented to the database server (mTLS)
    #[serde(default)]
    pub ssl_cert: Option<String>,
    // Path to the private key for ssl_cert (mTLS)
    #[serde(default)]
    pub ssl_key: Option<String>,
}

impl Default for Database {
    fn default() -> Self {
        Self {
            scheme: default_db_scheme(),
            database: default_db_database(),
            host: None,
            username: None,
            password: None,
            port: None,
            ssl_mode: None,
            ssl_root_cert: None,
            ssl_cert: None,
            ssl_key: None,
        }
    }
}

impl Database {
    pub const SSL_MODES: [&'static str; 5] =
        ["disable", "prefer", "require", "verify-ca", "verify-full"];

    /// Returns the SeaORM connection DSN for the configured backend.
    pub fn get_dsn(&self) -> String {
        self.build_dsn(false)
    }

    /// DSN with the password masked, safe for logging.
    pub fn get_redacted_dsn(&self) -> String {
        self.build_dsn(true)
    }

    /// Validates the TLS fields so a misconfigured deployment refuses to
    /// boot rather than silently connecting without the intended TLS.
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        let has_tls_material =
            self.ssl_root_cert.is_some() || self.ssl_cert.is_some() || self.ssl_key.is_some();
        if (has_tls_material || self.ssl_mode.is_some())
            && !matches!(self.scheme.as_str(), "mysql" | "postgres" | "postgresql")
        {
            return Err(anyhow!(
                "database.ssl_* options are set but scheme {:?} does not support TLS; use mysql, postgres, or postgresql",
                self.scheme
            ));
        }
        if let Some(mode) = self.ssl_mode.as_deref() {
            if !Self::SSL_MODES.contains(&mode) {
                return Err(anyhow!(
                    "database.ssl_mode {mode:?} is invalid; supported: {}",
                    Self::SSL_MODES.join(", ")
                ));
            }
        }
        if self.ssl_cert.is_some() != self.ssl_key.is_some() {
            return Err(anyhow!(
                "database.ssl_cert and database.ssl_key must be set together"
            ));
        }
        for (name, value) in [
            ("ssl_root_cert", &self.ssl_root_cert),
            ("ssl_cert", &self.ssl_cert),
            ("ssl_key", &self.ssl_key),
        ] {
            if let Some(path) = value {
                if !std::path::Path::new(path).exists() {
                    return Err(anyhow!("database.{name} file not found at path: {path}"));
                }
            }
        }
        // Without an explicit ssl_mode sqlx defaults to opportunistic TLS
        // (prefer/preferred), which silently downgrades to plaintext, and a
        // pinned CA is only checked by the verify-* modes.
        if has_tls_material {
            match self.ssl_mode.as_deref() {
                None => {
                    return Err(anyhow!(
                        "database.ssl_mode must be set when TLS certificate paths are configured"
                    ));
                }
                Some(mode) => {
                    if self.ssl_root_cert.is_some()
                        && !matches!(mode, "verify-ca" | "verify-full")
                    {
                        return Err(anyhow!(
                            "database.ssl_root_cert is set but database.ssl_mode {mode:?} never verifies it; use verify-ca or verify-full"
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn build_dsn(&self, redact_password: bool) -> String {
        match self.scheme.as_str() {
            "sqlite" | "sqlite3" => {
                // The redacted variant is only ever a log string; it must not
                // touch the filesystem.
                let path = std::path::Path::new(&self.database);
                if !redact_password && !path.exists() {
                    match std::fs::File::create(&self.database) {
                        Ok(_) => {}
                        Err(_e) => {
                            panic!(
                                "Verify that {} exists and is writable. You may need to create this file.",
                                &self.database
                            );
                        }
                    }
                }
                format!("sqlite://{}", &self.database)
            }
            "mysql" => self.network_dsn(
                "mysql",
                3306,
                &[
                    (
                        "ssl-mode",
                        &self.ssl_mode.as_deref().map(Self::mysql_ssl_mode),
                    ),
                    ("ssl-ca", &self.ssl_root_cert.as_deref()),
                    ("ssl-cert", &self.ssl_cert.as_deref()),
                    ("ssl-key", &self.ssl_key.as_deref()),
                ],
                redact_password,
            ),
            "postgres" | "postgresql" => self.network_dsn(
                "postgres",
                5432,
                &[
                    ("sslmode", &self.ssl_mode.as_deref()),
                    ("sslrootcert", &self.ssl_root_cert.as_deref()),
                    ("sslcert", &self.ssl_cert.as_deref()),
                    ("sslkey", &self.ssl_key.as_deref()),
                ],
                redact_password,
            ),
            _ => format!("sqlite://{}", "/etc/bvc/bvc.sqlite3"),
        }
    }

    fn network_dsn(
        &self,
        scheme: &str,
        default_port: u32,
        tls_params: &[(&str, &Option<&str>)],
        redact_password: bool,
    ) -> String {
        let username =
            utf8_percent_encode(self.username.as_deref().unwrap_or(""), DSN_ENCODE_SET)
                .to_string();
        let password = match (self.password.as_deref(), redact_password) {
            (Some(_), true) => "***".to_string(),
            (Some(password), false) => {
                utf8_percent_encode(password, DSN_ENCODE_SET).to_string()
            }
            (None, _) => String::new(),
        };
        let host = self.host.clone().unwrap_or(String::from("127.0.0.1"));
        let port = self.port.unwrap_or(default_port);
        let database = utf8_percent_encode(&self.database, DSN_ENCODE_SET).to_string();

        let mut dsn = format!("{scheme}://{username}:{password}@{host}:{port}/{database}");
        let query: Vec<String> = tls_params
            .iter()
            .filter_map(|(key, value)| {
                value
                    .filter(|v| !v.trim().is_empty())
                    .map(|v| format!("{key}={}", utf8_percent_encode(v, DSN_ENCODE_SET)))
            })
            .collect();
        if !query.is_empty() {
            dsn.push('?');
            dsn.push_str(&query.join("&"));
        }
        dsn
    }

    /// sqlx-mysql uses a different ssl-mode vocabulary than sqlx-postgres;
    /// the config speaks the postgres vocabulary and is translated here.
    fn mysql_ssl_mode(mode: &str) -> &str {
        match mode {
            "disable" => "disabled",
            "prefer" => "preferred",
            "require" => "required",
            "verify-ca" => "verify_ca",
            "verify-full" => "verify_identity",
            other => other,
        }
    }
}
