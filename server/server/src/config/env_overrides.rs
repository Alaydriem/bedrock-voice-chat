use std::collections::HashMap;

use anyhow::anyhow;

use super::app::Acme;
use super::app::ApplicationConfig;
use super::app::BedrockServerEntry;
use super::app::Meridian;

/// Curated environment-variable overrides applied on top of a parsed
/// configuration. Precedence: env override > config value > serde default.
/// An unset or empty variable never touches the config; a malformed value is
/// a hard startup error.
pub struct EnvOverrides {
    vars: HashMap<String, String>,
}

impl EnvOverrides {
    /// Captures the full process environment.
    pub fn from_env() -> Self {
        Self::from_vars(std::env::vars().collect())
    }

    /// Uses an explicit variable map. This is the testable constructor —
    /// tests must never mutate process-global env state.
    pub fn from_vars(vars: HashMap<String, String>) -> Self {
        Self { vars }
    }

    pub fn apply(&self, mut config: ApplicationConfig) -> Result<ApplicationConfig, anyhow::Error> {
        self.apply_server(&mut config)?;
        self.apply_tls(&mut config);
        self.apply_database(&mut config)?;
        self.apply_meridian(&mut config)?;
        self.apply_acme(&mut config)?;
        self.apply_bedrock(&mut config)?;
        Ok(config)
    }

    /// Empty values are treated as unset so `FOO=` in a compose file does not
    /// blank out a configured value.
    fn get(&self, key: &str) -> Option<&str> {
        self.vars
            .get(key)
            .map(String::as_str)
            .filter(|v| !v.trim().is_empty())
    }

    fn get_u32(&self, key: &str) -> Result<Option<u32>, anyhow::Error> {
        match self.get(key) {
            None => Ok(None),
            Some(raw) => raw
                .parse::<u32>()
                .map(Some)
                .map_err(|_| anyhow!("{key} must be an integer port, got {raw:?}")),
        }
    }

    fn get_u16(&self, key: &str) -> Result<Option<u16>, anyhow::Error> {
        match self.get(key) {
            None => Ok(None),
            Some(raw) => raw
                .parse::<u16>()
                .map(Some)
                .map_err(|_| anyhow!("{key} must be an integer port, got {raw:?}")),
        }
    }

    fn get_bool(&self, key: &str) -> Result<Option<bool>, anyhow::Error> {
        match self.get(key) {
            None => Ok(None),
            Some(raw) => match raw.to_ascii_lowercase().as_str() {
                "true" => Ok(Some(true)),
                "false" => Ok(Some(false)),
                _ => Err(anyhow!("{key} must be \"true\" or \"false\", got {raw:?}")),
            },
        }
    }

    fn get_list(&self, key: &str) -> Option<Vec<String>> {
        self.get(key).map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
    }

    fn apply_server(&self, config: &mut ApplicationConfig) -> Result<(), anyhow::Error> {
        // BVC_SERVER is host:port. URL forms (http:// or https://) are the
        // clap-bound server_url for CLI client commands, not a listen address.
        if let Some(addr) = self.get("BVC_SERVER") {
            if !addr.starts_with("http://") && !addr.starts_with("https://") {
                if let Some((host, port_str)) = addr.rsplit_once(':') {
                    if let Ok(port) = port_str.parse::<u32>() {
                        config.server.listen = host.to_string();
                        config.server.port = port;
                    }
                }
            }
        }
        if let Some(port) = self.get_u32("BVC_QUIC_PORT")? {
            config.server.quic_port = port;
        }
        if let Some(token) = self.get("BVC_ACCESS_TOKEN") {
            config.server.minecraft.access_token = token.to_string();
        }
        if let Some(telemetry) = self.get_bool("BVC_TELEMETRY")? {
            config.server.features.telemetry = telemetry;
        }
        Ok(())
    }

    fn apply_tls(&self, config: &mut ApplicationConfig) {
        if let Some(v) = self.get("BVC_TLS_CERTIFICATE") {
            config.server.tls.certificate = v.to_string();
        }
        if let Some(v) = self.get("BVC_TLS_KEY") {
            config.server.tls.key = v.to_string();
        }
        if let Some(v) = self.get("BVC_TLS_CERTS_PATH") {
            config.server.tls.certs_path = v.to_string();
        }
        if let Some(names) = self.get_list("BVC_TLS_NAMES") {
            config.server.tls.names = names;
        }
        if let Some(ips) = self.get_list("BVC_TLS_IPS") {
            config.server.tls.ips = ips;
        }
    }

    fn apply_database(&self, config: &mut ApplicationConfig) -> Result<(), anyhow::Error> {
        if let Some(v) = self.get("BVC_DATABASE_SCHEME") {
            config.database.scheme = v.to_string();
        }
        if let Some(v) = self.get("BVC_DATABASE_DATABASE") {
            config.database.database = v.to_string();
        }
        if let Some(v) = self.get("BVC_DATABASE_HOST") {
            config.database.host = Some(v.to_string());
        }
        if let Some(port) = self.get_u32("BVC_DATABASE_PORT")? {
            config.database.port = Some(port);
        }
        if let Some(v) = self.get("BVC_DATABASE_USERNAME") {
            config.database.username = Some(v.to_string());
        }
        if let Some(v) = self.get("BVC_DATABASE_PASSWORD") {
            config.database.password = Some(v.to_string());
        }
        Ok(())
    }

    fn apply_acme(&self, config: &mut ApplicationConfig) -> Result<(), anyhow::Error> {
        let email = self.get("BVC_ACME_EMAIL");
        let provider = self.get("BVC_ACME_PROVIDER");
        let api_token = self.get("BVC_ACME_API_TOKEN");
        let directory = self.get("BVC_ACME_DIRECTORY");
        let domains = self.get_list("BVC_ACME_DOMAINS");
        let dns_url = self.get("BVC_ACME_DNS_URL");
        let dns_username = self.get("BVC_ACME_DNS_USERNAME");
        let dns_password = self.get("BVC_ACME_DNS_PASSWORD");
        let dns_subdomain = self.get("BVC_ACME_DNS_SUBDOMAIN");

        let any_set = email.is_some()
            || provider.is_some()
            || api_token.is_some()
            || directory.is_some()
            || domains.is_some()
            || dns_url.is_some()
            || dns_username.is_some()
            || dns_password.is_some()
            || dns_subdomain.is_some();
        if !any_set {
            return Ok(());
        }

        // Materializing from scratch requires the block's identity fields;
        // provider-specific completeness is validated at server startup.
        if config.server.tls.acme.is_none() {
            let mut missing = Vec::new();
            if email.is_none() {
                missing.push("BVC_ACME_EMAIL");
            }
            if provider.is_none() {
                missing.push("BVC_ACME_PROVIDER");
            }
            if !missing.is_empty() {
                return Err(anyhow!(
                    "ACME env configuration is incomplete; missing: {}",
                    missing.join(", ")
                ));
            }
            config.server.tls.acme = Some(Acme::default());
        }

        let acme = config.server.tls.acme.as_mut().expect("just materialized");
        if let Some(v) = email {
            acme.email = v.to_string();
        }
        if let Some(v) = provider {
            acme.provider = v.to_string();
        }
        if let Some(v) = api_token {
            acme.api_token = Some(v.to_string());
        }
        if let Some(v) = directory {
            acme.directory = v.to_string();
        }
        if let Some(v) = domains {
            acme.domains = Some(v);
        }
        if let Some(v) = dns_url {
            acme.server_url = Some(v.to_string());
        }
        if let Some(v) = dns_username {
            acme.username = Some(v.to_string());
        }
        if let Some(v) = dns_password {
            acme.password = Some(v.to_string());
        }
        if let Some(v) = dns_subdomain {
            acme.subdomain = Some(v.to_string());
        }
        Ok(())
    }

    fn apply_bedrock(&self, config: &mut ApplicationConfig) -> Result<(), anyhow::Error> {
        if let Some(enabled) = self.get_bool("BVC_BEDROCK_ENABLED")? {
            config.server.bedrock.enabled = enabled;
        }
        if let Some(port) = self.get_u16("BVC_BEDROCK_TRANSFER_PORT")? {
            config.server.bedrock.transfer_port = port;
        }
        // Comma-separated `Name@host[:port][@protocol]` entries. A set variable
        // replaces the config list wholesale, matching env > config precedence.
        if let Some(entries) = self.get_list("BVC_BEDROCK_SERVERS") {
            config.server.bedrock.servers = entries
                .iter()
                .map(|raw| {
                    BedrockServerEntry::from_compact(raw)
                        .map_err(|e| anyhow!("BVC_BEDROCK_SERVERS: {e}"))
                })
                .collect::<Result<Vec<_>, _>>()?;
        }
        Ok(())
    }

    fn apply_meridian(&self, config: &mut ApplicationConfig) -> Result<(), anyhow::Error> {
        let url = self.get("BVC_MERIDIAN_URL");
        let api_key = self.get("BVC_MERIDIAN_API_KEY");
        let instance_id = self.get("BVC_MERIDIAN_INSTANCE_ID");
        let name = self.get("BVC_MERIDIAN_NAME");
        let host = self.get("BVC_MERIDIAN_HOST");
        let backend = self.get("BVC_MERIDIAN_BACKEND");

        let any_set = [url, api_key, instance_id, name, host, backend]
            .iter()
            .any(Option::is_some);
        if !any_set {
            return Ok(());
        }

        let parse_instance_id = |raw: &str| -> Result<u16, anyhow::Error> {
            raw.parse::<u16>()
                .map_err(|_| anyhow!("BVC_MERIDIAN_INSTANCE_ID must be a u16, got {raw:?}"))
        };

        match config.server.meridian.as_mut() {
            Some(meridian) => {
                if let Some(v) = url {
                    meridian.url = v.to_string();
                }
                if let Some(v) = api_key {
                    meridian.api_key = v.to_string();
                }
                if let Some(v) = instance_id {
                    meridian.instance_id = parse_instance_id(v)?;
                }
                if let Some(v) = name {
                    meridian.name = v.to_string();
                }
                if let Some(v) = host {
                    meridian.host = Some(v.to_string());
                }
                if let Some(v) = backend {
                    meridian.backend = v.to_string();
                }
            }
            None => {
                // Materializing the block from scratch requires the full
                // required set; HOST is optional, matching the struct.
                let mut missing = Vec::new();
                if url.is_none() {
                    missing.push("BVC_MERIDIAN_URL");
                }
                if api_key.is_none() {
                    missing.push("BVC_MERIDIAN_API_KEY");
                }
                if instance_id.is_none() {
                    missing.push("BVC_MERIDIAN_INSTANCE_ID");
                }
                if name.is_none() {
                    missing.push("BVC_MERIDIAN_NAME");
                }
                if backend.is_none() {
                    missing.push("BVC_MERIDIAN_BACKEND");
                }
                if !missing.is_empty() {
                    return Err(anyhow!(
                        "Meridian env configuration is incomplete; missing: {}",
                        missing.join(", ")
                    ));
                }
                config.server.meridian = Some(Meridian {
                    url: url.unwrap().to_string(),
                    api_key: api_key.unwrap().to_string(),
                    instance_id: parse_instance_id(instance_id.unwrap())?,
                    name: name.unwrap().to_string(),
                    host: host.map(str::to_string),
                    backend: backend.unwrap().to_string(),
                });
            }
        }
        Ok(())
    }
}
