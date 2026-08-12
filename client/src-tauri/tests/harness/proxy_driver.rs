use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;

use bedrock_server::{
    BedrockServer, PlayerAuthInputConfig, ServerConfig, ServerConnection, StartGameConfig,
    TextPacketConfig,
};
use common::bedrock_protocol::Bytes;
use common::bedrock_protocol::protocol::codec::PacketDecode;
use common::bedrock_protocol::protocol::packets::PacketHeader;
use common::bedrock_protocol::protocol::packets::generated::ids;
use common::bedrock_protocol::protocol::packets::generated::misc::play_sound::{
    PlaySoundPacketAny, PlaySoundPacketV975, PlaySoundPacketV2168,
};
use common::bedrock_protocol::protocol::packets::generated::misc::text::TextPacket;
use common::bedrock_protocol::protocol::types::generated::TextPacketBody;
use common::bedrock_protocol::protocol::types::primitives::BlockPos;
use common::bedrock_protocol::version::ProtocolVersion;

use crate::harness::server::EmbeddedServer;

/// Fake upstream Bedrock server for proxy/relay e2e tests. Accepts each BVC
/// proxy's upstream dial, indexes the connection by the proxy's offline login
/// name, and drives clientbound StartGame / PlayerAuthInput / PlaySound per
/// actor. For the cross-server relay it also rebroadcasts the proxy-injected
/// `!bvcp` presence chat clientbound to every connection (the realm fan-out).
pub struct FakeBedrockUpstream {
    server: BedrockServer,
    version: ProtocolVersion,
    addr: SocketAddr,
    // World identity emitted in StartGame. The proxy derives `relay_world_uuid`
    // from this (blake3 over seed|level_id|world_name), so two upstreams with
    // different names produce two distinct relay worlds (cross-realm isolation).
    world_name: String,
    conns: HashMap<String, ServerConnection>,
}

// Driver methods are exercised selectively across the proxy/relay scenario tests.
impl FakeBedrockUpstream {
    pub async fn bind(version: ProtocolVersion) -> Self {
        Self::bind_named(version, "Bedrock Server").await
    }

    /// Bind a fake upstream whose StartGame carries `world_name` as both the
    /// level id and world name, so proxied players derive a `relay_world_uuid`
    /// distinct from a differently-named realm.
    pub async fn bind_named(version: ProtocolVersion, world_name: &str) -> Self {
        let port = EmbeddedServer::free_port_udp();
        let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
        let server = BedrockServer::bind(ServerConfig {
            bind: addr,
            ..Default::default()
        })
        .await
        .expect("bind fake upstream");
        Self {
            server,
            version,
            addr,
            world_name: world_name.to_string(),
            conns: HashMap::new(),
        }
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Accept one proxy's upstream dial; index it by the offline login name; return the name.
    pub async fn accept_player(&mut self) -> String {
        let conn = self.server.accept().await.expect("fake upstream accept");
        let name = conn.player().name.clone();
        self.conns.insert(name.clone(), conn);
        name
    }

    /// Establish the world (sets the proxy session's world id — relay/jukebox
    /// prereq). The StartGame's level/world name is this upstream's `world_name`.
    pub async fn start_game(&mut self, name: &str) {
        let world_name = self.world_name.clone();
        let conn = self.conns.get_mut(name).expect("known player");
        let mut pkt = StartGameConfig::for_version(self.version).into_packet();
        pkt.level_id = world_name.clone();
        pkt.world_name = world_name;
        conn.send_packet(&pkt).await.expect("send StartGame");
    }

    /// Drive the player's position via a clientbound PlayerAuthInput.
    pub async fn drive_position(&mut self, name: &str, x: f32, y: f32, z: f32) {
        let conn = self.conns.get_mut(name).expect("known player");
        conn.send_packet(
            &PlayerAuthInputConfig::for_version(self.version)
                .at(x, y, z, 0.0, 0.0)
                .into_packet(),
        )
        .await
        .expect("send PlayerAuthInput");
    }

    /// Jukebox insert (bvc:play) at a world block position.
    pub async fn play_sound(
        &mut self,
        name: &str,
        audio_id: &str,
        x: i32,
        y: i32,
        z: i32,
        dim: &str,
    ) {
        let pkt = self.play_packet(&format!("bvc:play:{audio_id}:{dim}"), x, y, z);
        self.conns
            .get_mut(name)
            .expect("known player")
            .send_packet(&pkt)
            .await
            .expect("send PlaySound play");
    }

    /// Jukebox eject (bvc:eject) at a world block position.
    pub async fn eject(&mut self, name: &str, x: i32, y: i32, z: i32, dim: &str) {
        let pkt = self.play_packet(&format!("bvc:eject:{dim}"), x, y, z);
        self.conns
            .get_mut(name)
            .expect("known player")
            .send_packet(&pkt)
            .await
            .expect("send PlaySound eject");
    }

    /// Control-plane action (bvc:ctl:<action>) carried on the PlaySound bus; the
    /// proxy consumes it silently and nothing is forwarded to the client. Group
    /// actions ride ServerBound to the BVC server; self/preference actions are
    /// applied locally via the control-action channel. `ctl` is the action grammar
    /// after the prefix, e.g. "mute:1" or "group:create".
    pub async fn play_ctl(&mut self, name: &str, ctl: &str) {
        let pkt = self.play_packet(&format!("bvc:ctl:{ctl}"), 0, 0, 0);
        self.conns
            .get_mut(name)
            .expect("known player")
            .send_packet(&pkt)
            .await
            .expect("send PlaySound ctl");
    }

    /// Realm fan-out for the relay presence proof: drain any pending serverbound
    /// packets on every connection, and for each `!bvcp <token>` chat the proxy
    /// injected, re-emit it clientbound (a Chat TextPacket) to EVERY connection —
    /// exactly what a vanilla server does when a member chats. The peer's client
    /// then observes the token and completes the mutual proof.
    ///
    /// Each connection is polled with a short timeout so an idle stream does not
    /// block the pump; the test calls this on a loop (see `RelayWorld::pump_presence`).
    pub async fn rebroadcast_presence_chat(&mut self) {
        const POLL: Duration = Duration::from_millis(30);
        let version = self.version;
        let names: Vec<String> = self.conns.keys().cloned().collect();

        let mut messages: Vec<String> = Vec::new();
        for name in &names {
            let conn = self.conns.get_mut(name).expect("known conn");
            // Drain whatever is buffered right now; stop on the first idle poll.
            loop {
                match tokio::time::timeout(POLL, conn.recv_raw()).await {
                    Ok(Ok(subs)) => {
                        for sub in subs {
                            if let Some(m) = Self::bvc_message_from_sub(version, sub) {
                                messages.push(m);
                            }
                        }
                    }
                    // Timeout (nothing pending) or recv error: nothing more to drain.
                    _ => break,
                }
            }
        }

        for message in messages {
            let pkt = TextPacketConfig::chat(&message).into_packet();
            for name in &names {
                let conn = self.conns.get_mut(name).expect("known conn");
                let _ = conn.send_packet(&pkt).await;
            }
        }
    }

    /// Extract a `!bvc…`-prefixed chat message (presence `!bvcp` or announce
    /// `!bvca`) verbatim from a serverbound sub-packet, if it is a TEXT packet
    /// carrying one. A real realm rebroadcasts all chat; this mirrors that for the
    /// BVC control lines. Non-TEXT and non-bvc chat return None.
    fn bvc_message_from_sub(version: ProtocolVersion, sub: Bytes) -> Option<String> {
        let message = Self::chat_message_from_sub(version, sub)?;
        if message.starts_with("!bvcp ") || message.starts_with("!bvca ") {
            Some(message)
        } else {
            None
        }
    }

    /// Extract any chat message from a serverbound sub-packet, if it is a TEXT
    /// packet. Non-TEXT sub-packets return None.
    fn chat_message_from_sub(version: ProtocolVersion, sub: Bytes) -> Option<String> {
        let mut buf = sub;
        let id = PacketHeader::read(&mut buf).ok()?;
        if id != ids::TEXT {
            return None;
        }
        let text = TextPacket::decode_for(version, &mut buf).ok()?;
        Some(match text.body {
            TextPacketBody::MessageOnly(b) => b.message,
            TextPacketBody::AuthorAndMessage(b) => b.message,
            TextPacketBody::MessageAndParams(b) => b.message,
        })
    }

    /// Await a serverbound `!bvcs:` state-ride chat from `name`'s proxy session
    /// that passes `pred`, draining that connection's serverbound stream until
    /// one arrives or `timeout` elapses.
    pub async fn await_bvcs<F>(
        &mut self,
        name: &str,
        pred: F,
        timeout: Duration,
    ) -> Option<String>
    where
        F: Fn(&str) -> bool,
    {
        let version = self.version;
        let deadline = tokio::time::Instant::now() + timeout;
        let conn = self.conns.get_mut(name).expect("known player");
        loop {
            let now = tokio::time::Instant::now();
            if now >= deadline {
                return None;
            }
            match tokio::time::timeout(deadline - now, conn.recv_raw()).await {
                Ok(Ok(subs)) => {
                    for sub in subs {
                        if let Some(m) = Self::chat_message_from_sub(version, sub) {
                            if m.starts_with("!bvcs:") && pred(&m) {
                                return Some(m);
                            }
                        }
                    }
                }
                // Timeout or a closed upstream stream: no ride observed.
                _ => return None,
            }
        }
    }

    /// Every serverbound chat message `name`'s proxy session sends within
    /// `window`, in arrival order.
    ///
    /// Collecting rather than matching is what lets a caller assert an absence:
    /// `await_bvcs` answers "did this arrive", which cannot distinguish a ride
    /// that was suppressed from one that was merely slow. This drains the whole
    /// window every time, so an empty result means the window really was silent.
    pub async fn drain_serverbound_chat(&mut self, name: &str, window: Duration) -> Vec<String> {
        let version = self.version;
        let deadline = tokio::time::Instant::now() + window;
        let conn = self.conns.get_mut(name).expect("known player");
        let mut seen = vec![];

        loop {
            let now = tokio::time::Instant::now();
            if now >= deadline {
                return seen;
            }
            match tokio::time::timeout(deadline - now, conn.recv_raw()).await {
                Ok(Ok(subs)) => {
                    for sub in subs {
                        if let Some(m) = Self::chat_message_from_sub(version, sub) {
                            seen.push(m);
                        }
                    }
                }
                // Timeout closes the window; a closed upstream ends it early with
                // whatever was already observed.
                _ => return seen,
            }
        }
    }

    /// Build the PlaySound variant matching the negotiated peer version. The
    /// `versioned_codec_dispatch!` macro routes each version to its own codec, and
    /// a codec silently encodes a DEFAULT-ZERO packet (empty name) for any other
    /// variant — so the variant must track the version or the proxy sees garbage.
    /// Position follows the Bedrock 1/8-block fixed-point convention: the wire
    /// value is world_coord * 8 (handler divides by 8 to recover block coords).
    fn play_packet(&self, name: &str, x: i32, y: i32, z: i32) -> PlaySoundPacketAny {
        let position = BlockPos::new(x * 8, y * 8, z * 8);
        if self.version >= ProtocolVersion::V2168 {
            PlaySoundPacketAny::V2168(PlaySoundPacketV2168 {
                name: name.to_string(),
                position,
                volume: 1.0,
                pitch: 1.0,
                loop_count: 0,
                server_sound_handle: None,
            })
        } else {
            PlaySoundPacketAny::V975(PlaySoundPacketV975 {
                name: name.to_string(),
                position,
                volume: 1.0,
                pitch: 1.0,
                server_sound_handle: None,
            })
        }
    }
}
