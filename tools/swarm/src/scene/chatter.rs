use std::time::Duration;

use futures_util::SinkExt;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::error_chain::ErrorChain;
use crate::scene::{ChatBeat, ChatScript, ChatWireFrame, SceneConfig};

/// Feeds a world's chat over `/api/websocket/chat`, the way a game mod does.
///
/// A mod can set request headers, so identity here is the access token on the upgrade and no
/// ticket is involved. No subprotocol is offered, and none may be: the server returns a bare
/// channel for exactly that reason, and a client that names one gets an unparseable handshake
/// back rather than a rejection.
pub struct Chatter {
    url: String,
    access_token: String,
    world: String,
    world_name: String,
    period: Duration,
}

impl Chatter {
    pub fn new(config: &SceneConfig, period: Duration) -> Result<Self, anyhow::Error> {
        // Delivery is addressed by matching a player's own `world_uuid` against the room's, so
        // a scene whose players declare no world has nobody to deliver to. The feed stages
        // everyone with this one value, which is also why it filters nobody out of proximity.
        let world = config.world_uuid.clone().ok_or_else(|| {
            anyhow::anyhow!(
                "chat needs `world_uuid` set in the scene config: the server delivers a line \
                 only to players whose world matches the room's"
            )
        })?;

        // This socket trusts the webpki roots and nothing else. Honouring a private CA would
        // need its own TLS connector, and refusing is better than quietly verifying the chat
        // connection differently from every other connection the tool makes.
        if config.ca.is_some() {
            return Err(anyhow::anyhow!(
                "chat cannot use the `ca` in this config — it trusts public roots only. Point \
                 `server` at a publicly trusted name, or run the scene without --chat"
            ));
        }

        let base = config.server_base();
        let url = if let Some(host) = base.strip_prefix("https://") {
            format!("wss://{}/api/websocket/chat", host)
        } else if let Some(host) = base.strip_prefix("http://") {
            format!("ws://{}/api/websocket/chat", host)
        } else {
            return Err(anyhow::anyhow!(
                "`server` must start with https:// or http://, found {}",
                base
            ));
        };

        Ok(Self {
            url,
            access_token: config.access_token.clone(),
            world,
            world_name: config.world_name.clone(),
            period,
        })
    }

    /// Connect, announce the world, then read the script aloud until the task is aborted.
    ///
    /// Connected before returning so a rejected token fails the scene rather than leaving a
    /// chat log running that nobody receives.
    pub async fn hold(
        self,
        mut script: ChatScript,
    ) -> Result<tokio::task::JoinHandle<()>, anyhow::Error> {
        let mut request = self
            .url
            .as_str()
            .into_client_request()
            .map_err(|e| anyhow::anyhow!("building chat request for {}: {}", self.url, e))?;

        let bearer = format!("Bearer {}", self.access_token);
        request.headers_mut().insert(
            "Authorization",
            HeaderValue::from_str(&bearer)
                .map_err(|e| anyhow::anyhow!("access_token is not a valid header value: {}", e))?,
        );

        let (mut stream, _) = tokio_tungstenite::connect_async(request).await.map_err(|e| {
            anyhow::anyhow!(
                "connecting chat socket to {}: {}",
                self.url,
                ErrorChain::of(&e)
            )
        })?;

        Self::send(
            &mut stream,
            &ChatWireFrame::Hello {
                world: self.world.clone(),
                world_name: self.world_name.clone(),
                game: "minecraft".to_string(),
                worlds: Vec::new(),
            },
        )
        .await?;

        Ok(tokio::spawn(async move {
            let mut ticker = tokio::time::interval(self.period);
            loop {
                ticker.tick().await;

                let Some(beat) = script.next() else {
                    continue;
                };

                let frame = match beat {
                    ChatBeat::Chat { author, text } => {
                        println!("  <{}> {}", author, text);
                        ChatWireFrame::Chat { author, text }
                    }
                    ChatBeat::Event { text } => {
                        println!("  * {}", text);
                        ChatWireFrame::Event { text }
                    }
                };

                if let Err(e) = Self::send(&mut stream, &frame).await {
                    eprintln!("[scene] chat socket closed: {}", e);
                    break;
                }
            }
        }))
    }

    async fn send(
        stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        frame: &ChatWireFrame,
    ) -> Result<(), anyhow::Error> {
        let body =
            serde_json::to_string(frame).map_err(|e| anyhow::anyhow!("encoding chat frame: {}", e))?;
        stream
            .send(Message::Text(body.into()))
            .await
            .map_err(|e| anyhow::anyhow!("sending chat frame: {}", e))?;
        Ok(())
    }
}
