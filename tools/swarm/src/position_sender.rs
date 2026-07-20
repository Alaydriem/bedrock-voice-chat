use serde_json::json;

/// Posts bot positions to `/api/position` so the server's player cache contains
/// them — without this, `route_audio_frame` drops every recipient and no bot
/// hears anything. Coordinates mirror `route_bench`: clusters 10 km apart,
/// members 5 blocks apart, so each host-local group is mutually in range and
/// distinct groups never bleed via proximity.
pub struct PositionSender {
    client: reqwest::Client,
    url: String,
    access_token: String,
    // (gamertag, x, y, z)
    players: Vec<(String, f32, f32, f32)>,
}

impl PositionSender {
    pub fn new(
        server: &str,
        access_token: String,
        ca_pem: Option<&str>,
        names_with_cluster: &[(String, usize)],
        group_size: usize,
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

        let players = names_with_cluster
            .iter()
            .map(|(name, cluster)| {
                let within = (*cluster % group_size.max(1)) as f32;
                let cluster_base = (*cluster / group_size.max(1)) as f32;
                (name.clone(), cluster_base * 10_000.0 + within * 5.0, 64.0, 0.0)
            })
            .collect();

        Ok(Self {
            client,
            url: format!("{}/api/position", server.trim_end_matches('/')),
            access_token,
            players,
        })
    }

    /// POST all bot positions once. Returns Err on transport/HTTP failure so the
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
}
