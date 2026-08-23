use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use crate::scene::{Placement, SceneConfig};

/// Advertises the staged roster to `/api/position`, continuously, the way a game mod
/// does.
///
/// Continuously is the operative word: the server's presence cache holds a player for
/// 15 seconds after their last report, so a single post produces a roster that fills
/// in and then empties again while you are still framing the shot.
///
/// The observer is advertised alongside everyone else. The position feed answers
/// nothing for an observer it cannot locate in the world — an authenticated client not
/// yet in a game is a normal state, and it reads as an empty ring — so staging the
/// observer is what lets a scene compose with no Minecraft session running at all.
pub struct PresenceFeed {
    client: reqwest::Client,
    url: String,
    access_token: String,
    body: Value,
    interval: Duration,
    advertised: usize,
}

impl PresenceFeed {
    pub fn new(
        config: &SceneConfig,
        staged: &[Placement],
        ca_pem: Option<&str>,
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

        let mut players = vec![Self::observer_entry(config)];
        players.extend(staged.iter().map(|p| Self::staged_entry(config, p)));

        Ok(Self {
            client,
            url: format!("{}/api/position", config.server_base()),
            access_token: config.access_token.clone(),
            advertised: players.len(),
            body: json!({ "game": "minecraft", "players": players }),
            interval: Duration::from_micros(1_000_000 / u64::from(config.position_hz.max(1))),
        })
    }

    pub fn advertised(&self) -> usize {
        self.advertised
    }

    /// Post the roster once, failing loudly.
    ///
    /// The first post is awaited before anything else starts, because a rejected token
    /// and an empty world look identical in the app and only one of them is worth
    /// debugging in the app.
    pub async fn post_once(&self) -> Result<(), anyhow::Error> {
        let response = self
            .client
            .post(&self.url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(&self.body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("posting positions to {}: {}", self.url, e))?;

        let status = response.status();
        if status.is_success() {
            return Ok(());
        }

        Err(anyhow::anyhow!(
            "posting positions returned {} — {}",
            status.as_u16(),
            match status.as_u16() {
                401 | 403 => "check `access_token` against the server's minecraft.access_token",
                _ => "check `server` and that the server is up",
            }
        ))
    }

    /// Hold the roster up until the returned handle is dropped or aborted.
    pub async fn hold(self: Arc<Self>) -> Result<tokio::task::JoinHandle<()>, anyhow::Error> {
        self.post_once().await?;

        Ok(tokio::spawn(async move {
            let mut ticker = tokio::time::interval(self.interval);
            loop {
                ticker.tick().await;
                if let Err(e) = self.post_once().await {
                    eprintln!("[scene] {}", e);
                }
            }
        }))
    }

    // Origin, yaw 0. Every staged bearing is relative to this, so it is not an
    // arbitrary choice — moving it would rotate the whole composition.
    fn observer_entry(config: &SceneConfig) -> Value {
        Self::entry(config, &config.observer, (0.0, config.origin_y, 0.0), false)
    }

    fn staged_entry(config: &SceneConfig, placement: &Placement) -> Value {
        Self::entry(
            config,
            &placement.name,
            placement.coordinates(config.origin_y),
            true,
        )
    }

    // `bridged_voice` is what makes a staged player read as a voice participant rather
    // than as somebody merely present in the world. It is the flag a mod that holds a
    // player's voice connection itself sets, and the position feed is the only thing
    // that consumes it — so setting it changes presentation and touches no audio path.
    //
    // The observer is exempt: their own entry is never rendered as a participant, and
    // claiming a bridged connection for the person holding a real one is a lie with no
    // upside.
    fn entry(config: &SceneConfig, name: &str, at: (f32, f32, f32), bridged: bool) -> Value {
        let (x, y, z) = at;
        let mut entry = json!({
            "name": name,
            "coordinates": { "x": x, "y": y, "z": z },
            "orientation": { "x": 0.0, "y": 0.0 },
            "dimension": config.dimension,
            "deafen": false,
            "spectator": false,
            "bridged_voice": bridged,
        });

        if let Some(world) = &config.world_uuid {
            entry["world_uuid"] = json!(world);
        }

        entry
    }
}
