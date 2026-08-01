use std::time::Duration;

use serde_json::json;

/// Advertises the whole simulated realm to `/api/position`, continuously, the
/// way a game mod does.
///
/// Two properties of the real feed are load-bearing and were previously absent:
///
/// * **The entire roster goes in ONE request.** Position datagram size is driven
///   by players-per-request, not by how many bots are on voice, so sharding the
///   roster across agents pins that axis at the per-agent bot count and no
///   amount of total scale ever exercises it. The controller owns this sender
///   for that reason — it is the only party that knows every name.
/// * **Every player carries a `world_uuid`**, which is a large share of
///   per-player encoded size. Omitting it understates datagram size roughly 2x.
///
/// Coordinates mirror `route_bench`: clusters 10 km apart, members 5 blocks
/// apart, so each voice group is mutually in range and distinct groups never
/// bleed via proximity. Filler players are parked far from every cluster so
/// they add roster weight without joining anyone's audio.
pub struct PositionSender {
    client: reqwest::Client,
    url: String,
    access_token: String,
    world_uuid: String,
    // (gamertag, x, y, z)
    players: Vec<(String, f32, f32, f32)>,
}

impl PositionSender {
    // Far enough from any cluster that filler never lands in a bot's range.
    const FILLER_ORIGIN: f32 = 5_000_000.0;

    pub fn new(
        server: &str,
        access_token: String,
        ca_pem: Option<&str>,
        world_uuid: String,
        names_with_cluster: &[(String, usize)],
        group_size: usize,
        filler_names: &[String],
    ) -> Result<Self, anyhow::Error> {
        let mut builder = reqwest::Client::builder().use_rustls_tls();
        if let Some(ca) = ca_pem {
            let ca_cert = reqwest::Certificate::from_pem(ca.as_bytes())
                .map_err(|e| anyhow::anyhow!("parsing ca pem: {}", e))?;
            builder = builder.add_root_certificate(ca_cert);
        }
        let client = builder
            .build()
            .map_err(|e| anyhow::anyhow!("building position client: {}", e))?;

        let mut players: Vec<(String, f32, f32, f32)> = names_with_cluster
            .iter()
            .map(|(name, cluster)| {
                let within = (*cluster % group_size.max(1)) as f32;
                let cluster_base = (*cluster / group_size.max(1)) as f32;
                (name.clone(), cluster_base * 10_000.0 + within * 5.0, 64.0, 0.0)
            })
            .collect();

        players.extend(filler_names.iter().enumerate().map(|(i, name)| {
            (
                name.clone(),
                Self::FILLER_ORIGIN + i as f32 * 100.0,
                64.0,
                0.0,
            )
        }));

        Ok(Self {
            client,
            url: format!("{}/api/position", server.trim_end_matches('/')),
            access_token,
            world_uuid,
            players,
        })
    }

    /// Players advertised per request — the number that drives datagram size.
    pub fn advertised_players(&self) -> usize {
        self.players.len()
    }

    /// POST the full roster once. Returns Err on transport/HTTP failure so the
    /// caller can surface a misconfigured token (positions are load-bearing).
    pub async fn post_once(&self) -> Result<(), anyhow::Error> {
        let players: Vec<serde_json::Value> = self
            .players
            .iter()
            .map(|(name, x, y, z)| {
                json!({
                    "name": name,
                    "coordinates": { "x": x, "y": y, "z": z },
                    "orientation": { "x": 0.0, "y": 0.0 },
                    "dimension": "overworld",
                    "deafen": false,
                    "spectator": false,
                    "world_uuid": self.world_uuid,
                })
            })
            .collect();

        let resp = self
            .client
            .post(&self.url)
            .header("X-MC-Access-Token", &self.access_token)
            .json(&json!({ "game": "minecraft", "players": players }))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("post positions: {}", e))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "post positions returned {} (check access_token)",
                resp.status().as_u16()
            ))
        }
    }

    /// Advertise continuously at `hz` until the returned task is aborted. A
    /// single successful post is required first so a bad token fails the run
    /// before any container is launched.
    pub async fn spawn_advertising(
        self: std::sync::Arc<Self>,
        hz: u32,
    ) -> Result<tokio::task::JoinHandle<()>, anyhow::Error> {
        self.post_once().await?;

        let interval = Duration::from_micros(1_000_000 / u64::from(hz.max(1)));

        Ok(tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                if let Err(e) = self.post_once().await {
                    eprintln!("[controller] position advertise failed: {}", e);
                }
            }
        }))
    }
}
