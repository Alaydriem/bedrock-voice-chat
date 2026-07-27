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

        tracing::info!(
            url = %self.config.url,
            name = %name,
            hostname = %self.hostname,
            tcp_addr = %tcp_addr,
            udp_addr = %udp_addr,
            instance_id = self.config.instance_id,
            "Registering with Meridian"
        );

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

        tracing::debug!(name = %name, "Refreshed Meridian registration");
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
                            tracing::warn!(
                                error = %e,
                                name = %self.record_name(),
                                "Meridian heartbeat failed; will retry"
                            );
                        }
                    }
                }
            }
            tracing::info!("Meridian heartbeat stopped");
        })
    }
}

/// How often the registry record is refreshed.
///
/// Must stay well below Meridian's lease TTL so a couple of consecutive failures
/// do not expire a healthy record.
pub const HEARTBEAT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(15);

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use tokio_util::sync::CancellationToken;

    use super::*;

    fn config(name: &str) -> Meridian {
        Meridian {
            // Closed port: every registration attempt will fail.
            url: "https://127.0.0.1:1".to_string(),
            api_key: "k".to_string(),
            instance_id: 42,
            name: name.to_string(),
            host: None,
            backend: "127.0.0.1".to_string(),
        }
    }

    fn service(name: &str) -> MeridianService {
        MeridianService::new(
            config(name),
            "127.0.0.1".to_string(),
            443,
            4433,
            "x.example.com".to_string(),
        )
    }

    #[test]
    fn record_name_comes_from_config_and_is_stable() {
        let a = service("customer-x");
        let b = service("customer-x");
        assert_eq!(a.record_name(), "customer-x");
        assert_eq!(
            a.record_name(),
            b.record_name(),
            "two services with the same config must use the same record name, \
             otherwise re-registration leaks a registry entry per restart"
        );
    }

    #[tokio::test]
    async fn heartbeat_survives_registration_failures() {
        let svc = Arc::new(service("customer-x"));
        let shutdown = CancellationToken::new();
        let handle = svc.spawn_heartbeat(shutdown.clone());

        // Long enough for at least one tick against the closed port.
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(
            !handle.is_finished(),
            "the heartbeat must survive failures — a Meridian restart is exactly \
             the case it exists for"
        );

        shutdown.cancel();
        tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .expect("heartbeat must stop on shutdown")
            .unwrap();
    }
}
