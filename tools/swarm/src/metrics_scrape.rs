/// A point-in-time reading of the server's audio-routing metrics, parsed from
/// the Prometheus text exposition at `/metrics/` (a guard-free route — only the
/// CA is needed for TLS trust).
#[derive(Debug, Clone, Copy, Default)]
pub struct MetricsSnapshot {
    pub frames_routed: u64,
    pub recipient_drops: u64,
    pub route_duration_sum: f64,
    pub route_duration_count: u64,
}

impl MetricsSnapshot {
    /// Mean per-frame route duration in microseconds over this snapshot's window,
    /// or 0 when no frames were recorded.
    pub fn mean_route_us(&self) -> f64 {
        if self.route_duration_count == 0 {
            0.0
        } else {
            self.route_duration_sum / self.route_duration_count as f64 * 1_000_000.0
        }
    }

    /// Field-wise delta (`self` after − `before`), for the mean over the run window.
    pub fn delta(&self, before: &MetricsSnapshot) -> MetricsSnapshot {
        MetricsSnapshot {
            frames_routed: self.frames_routed.saturating_sub(before.frames_routed),
            recipient_drops: self.recipient_drops.saturating_sub(before.recipient_drops),
            route_duration_sum: self.route_duration_sum - before.route_duration_sum,
            route_duration_count: self
                .route_duration_count
                .saturating_sub(before.route_duration_count),
        }
    }
}

/// Fetches and parses `/metrics/` for the audio-routing families.
pub struct MetricsScrape {
    client: reqwest::Client,
    url: String,
}

impl MetricsScrape {
    pub fn new(server: &str, ca_pem: Option<&str>) -> Result<Self, anyhow::Error> {
        let mut builder = reqwest::Client::builder().use_rustls_tls();
        if let Some(ca) = ca_pem {
            let ca_cert = reqwest::Certificate::from_pem(ca.as_bytes())
                .map_err(|e| anyhow::anyhow!("parsing ca pem: {}", e))?;
            builder = builder.add_root_certificate(ca_cert);
        }
        let client = builder
            .build()
            .map_err(|e| anyhow::anyhow!("building metrics client: {}", e))?;
        Ok(Self {
            client,
            url: format!("{}/metrics/", server.trim_end_matches('/')),
        })
    }

    pub async fn snapshot(&self) -> Result<MetricsSnapshot, anyhow::Error> {
        let text = self
            .client
            .get(&self.url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("GET {}: {}", self.url, e))?
            .text()
            .await
            .map_err(|e| anyhow::anyhow!("reading {} body: {}", self.url, e))?;
        Ok(Self::parse(&text))
    }

    fn parse(text: &str) -> MetricsSnapshot {
        let mut snap = MetricsSnapshot::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((name, value)) = line.rsplit_once(' ') else {
                continue;
            };
            match name {
                "bvc_audio_frames_routed_total" => {
                    snap.frames_routed = Self::as_u64(value);
                }
                "bvc_audio_route_recipient_drops_total" => {
                    snap.recipient_drops = Self::as_u64(value);
                }
                "bvc_audio_route_duration_seconds_sum" => {
                    snap.route_duration_sum = value.parse().unwrap_or(0.0);
                }
                "bvc_audio_route_duration_seconds_count" => {
                    snap.route_duration_count = Self::as_u64(value);
                }
                _ => {}
            }
        }
        snap
    }

    // Prometheus counters render as floats (e.g. "1234" or "1234.0").
    fn as_u64(value: &str) -> u64 {
        value.parse::<f64>().unwrap_or(0.0) as u64
    }
}
