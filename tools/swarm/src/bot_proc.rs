use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::json;
use tokio::process::{Child, ChildStdin, Command};

use crate::bridge_codec::BridgeCodec;

/// Live state of one bot, folded from its stdout `OutMsg` stream by a background
/// task. `frames_received` is the client's `frames_from_quic` counter.
#[derive(Default, Clone)]
pub struct BotState {
    pub ready: bool,
    pub connected: bool,
    pub disconnected: bool,
    pub channel_id: Option<String>,
    pub frames_sent: u64,
    pub frames_received: u64,
}

/// One `bvc_client_e2e` child process: spawned with `BVC_E2E_*` env, its stdout
/// drained into shared `BotState`, its stdin held for PCM/stats/shutdown frames.
pub struct BotProc {
    name: String,
    child: Child,
    stdin: ChildStdin,
    state: Arc<Mutex<BotState>>,
}

impl BotProc {
    /// Spawn a bot. `channel` create-or-joins a channel by name; pass
    /// `channel_id` instead to join a known channel directly (group followers).
    pub fn spawn(
        bin: &str,
        gamertag: &str,
        code: &str,
        server: &str,
        channel: &str,
        channel_id: Option<&str>,
    ) -> Result<Self, anyhow::Error> {
        let mut cmd = Command::new(bin);
        cmd.env("BVC_E2E_SERVER", server)
            .env("BVC_E2E_GAMERTAG", gamertag)
            .env("BVC_E2E_CODE", code)
            .env("BVC_E2E_CHANNEL", channel)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        if let Some(id) = channel_id {
            cmd.env("BVC_E2E_CHANNEL_ID", id);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| anyhow::anyhow!("spawn bot {} ({}): {}", gamertag, bin, e))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("bot {} stdin not piped", gamertag))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("bot {} stdout not piped", gamertag))?;

        let state = Arc::new(Mutex::new(BotState::default()));
        let reader_state = state.clone();
        tokio::spawn(async move {
            let mut stdout = stdout;
            loop {
                match BridgeCodec::read(&mut stdout).await {
                    Ok(value) => Self::fold(&reader_state, &value),
                    Err(_) => {
                        reader_state.lock().unwrap().disconnected = true;
                        break;
                    }
                }
            }
        });

        Ok(Self {
            name: gamertag.to_string(),
            child,
            stdin,
            state,
        })
    }

    fn fold(state: &Arc<Mutex<BotState>>, value: &serde_json::Value) {
        let mut s = state.lock().unwrap();
        if let Some(tag) = value.as_str() {
            match tag {
                "Ready" => s.ready = true,
                "Connected" => s.connected = true,
                "Disconnected" => s.disconnected = true,
                _ => {}
            }
            return;
        }
        if let Some(cj) = value.get("ChannelJoined") {
            s.connected = true;
            if let Some(id) = cj.get("channel_id").and_then(|v| v.as_str()) {
                s.channel_id = Some(id.to_string());
            }
        } else if let Some(stats) = value.get("Stats") {
            if let Some(n) = stats.get("frames_sent").and_then(|v| v.as_u64()) {
                s.frames_sent = n;
            }
            if let Some(n) = stats.get("frames_from_quic").and_then(|v| v.as_u64()) {
                s.frames_received = n;
            }
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn snapshot(&self) -> BotState {
        self.state.lock().unwrap().clone()
    }

    /// Block up to `timeout` for the bot to report a joined channel (which also
    /// implies connected). Returns the channel id if it arrived.
    pub async fn await_channel(&self, timeout: Duration) -> Option<String> {
        let deadline = Instant::now() + timeout;
        loop {
            {
                let s = self.state.lock().unwrap();
                if let Some(id) = &s.channel_id {
                    return Some(id.clone());
                }
                if s.disconnected {
                    return None;
                }
            }
            if Instant::now() >= deadline {
                return None;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    pub async fn feed(&mut self, samples: Vec<f32>) -> std::io::Result<()> {
        BridgeCodec::write(&mut self.stdin, &json!({ "InputPcm": { "samples": samples } })).await
    }

    pub async fn request_stats(&mut self) -> std::io::Result<()> {
        BridgeCodec::write(&mut self.stdin, &json!("RequestStats")).await
    }

    pub async fn shutdown(&mut self) {
        let _ = BridgeCodec::write(&mut self.stdin, &json!("Shutdown")).await;
        let _ = self.child.wait().await;
    }
}
