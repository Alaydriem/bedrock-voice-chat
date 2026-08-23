use common::curia;
use crate::config::Meridian;

pub struct MeridianService {
    config: Meridian,
    backend: String,
    tls_port: u32,
    quic_port: u32,
    hostname: String,
}

impl MeridianService {
    pub fn new(
        config: Meridian,
        backend: String,
        tls_port: u32,
        quic_port: u32,
        hostname: String,
    ) -> Self {
        Self {
            config,
            backend,
            tls_port,
            quic_port,
            hostname,
        }
    }

    /// Stable registry record name.
    ///
    /// Must come from config, not be generated per call: a fresh name on every
    /// registration means Meridian's `by_name` index accumulates a dead entry on
    /// every BVC restart, and nothing ever reclaims it.
    pub fn record_name(&self) -> &str {
        &self.config.name
    }

    pub async fn register(&self) -> Result<(), anyhow::Error> {
        let name = self.record_name().to_string();
        let tcp_addr = format!("{}:{}", self.backend, self.tls_port);
        let udp_addr = format!("{}:{}", self.backend, self.quic_port);

        curia::info!("Registering with Meridian", { "url": self.config.url.to_string(), "name": name.to_string(), "hostname": self.hostname.to_string(), "tcp_addr": tcp_addr.to_string(), "udp_addr": udp_addr.to_string(), "instance_id": self.config.instance_id });

        let client = meridian::api::MeridianClient::builder(&self.config.url, &self.config.api_key)
            .build()?;

        // `update_backend` is an upsert on Meridian's side, so this both registers
        // and refreshes. `register` (POST) would conflict on the second call.
        client
            .update_backend(
                &name,
                &self.hostname,
                tcp_addr,
                udp_addr,
                self.config.instance_id,
            )
            .await?;

        curia::debug!("Refreshed Meridian registration", { "name": name.to_string() });
        Ok(())
    }

    /// Refresh the registry record forever, every [`HEARTBEAT_INTERVAL`].
    ///
    /// Registering once at startup is not enough: if Meridian restarts, or this
    /// record's lease lapses, a one-shot registration leaves this customer
    /// unroutable until BVC itself restarts. Errors are logged and retried,
    /// because a failure means Meridian is unreachable — precisely when giving up
    /// would be worst.
    pub fn spawn_heartbeat(
        self: std::sync::Arc<Self>,
        shutdown: tokio_util::sync::CancellationToken,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(HEARTBEAT_INTERVAL);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tokio::select! {
                    _ = shutdown.cancelled() => break,
                    _ = ticker.tick() => {
                        if let Err(e) = self.register().await {
                            curia::warn!("Meridian heartbeat failed; will retry", { "error": e.to_string(), "name": self.record_name().to_string() });
                        }
                    }
                }
            }
            curia::info!("Meridian heartbeat stopped");
        })
    }
}

/// How often the registry record is refreshed.
///
/// Must stay well below Meridian's lease TTL so a couple of consecutive failures
/// do not expire a healthy record.
pub const HEARTBEAT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(15);
