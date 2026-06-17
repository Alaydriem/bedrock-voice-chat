use std::collections::HashMap;
use std::net::SocketAddr;

use common::bedrock_protocol::protocol::packets::generated::misc::play_sound::{
    PlaySoundPacketAny, PlaySoundPacketV975,
};
use common::bedrock_protocol::protocol::types::primitives::BlockPos;
use common::bedrock_protocol::version::ProtocolVersion;
use bedrock_server::{BedrockServer, PlayerAuthInputConfig, ServerConfig, ServerConnection, StartGameConfig};

use crate::harness::server::EmbeddedServer;

/// Fake upstream Bedrock server for proxy e2e tests. Accepts each BVC proxy's
/// upstream dial, indexes the connection by the proxy's offline login name, and
/// drives clientbound StartGame / PlayerAuthInput / PlaySound per actor.
pub struct FakeBedrockUpstream {
    server: BedrockServer,
    version: ProtocolVersion,
    addr: SocketAddr,
    conns: HashMap<String, ServerConnection>,
}

// Driver methods are exercised selectively across the proxy scenario tests.
#[allow(dead_code)]
impl FakeBedrockUpstream {
    pub async fn bind(version: ProtocolVersion) -> Self {
        let port = EmbeddedServer::free_port_udp();
        let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
        let server = BedrockServer::bind(ServerConfig { bind: addr, ..Default::default() })
            .await
            .expect("bind fake upstream");
        Self { server, version, addr, conns: HashMap::new() }
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

    /// Establish the world (sets the proxy session's world_uuid — jukebox prereq).
    pub async fn start_game(&mut self, name: &str) {
        let conn = self.conns.get_mut(name).expect("known player");
        conn.send_packet(&StartGameConfig::for_version(self.version).into_packet())
            .await
            .expect("send StartGame");
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
    pub async fn play_sound(&mut self, name: &str, audio_id: &str, x: i32, y: i32, z: i32, dim: &str) {
        let pkt = Self::play_packet(&format!("bvc:play:{audio_id}:{dim}"), x, y, z);
        self.conns
            .get_mut(name)
            .expect("known player")
            .send_packet(&pkt)
            .await
            .expect("send PlaySound play");
    }

    /// Jukebox eject (bvc:eject) at a world block position.
    pub async fn eject(&mut self, name: &str, x: i32, y: i32, z: i32, dim: &str) {
        let pkt = Self::play_packet(&format!("bvc:eject:{dim}"), x, y, z);
        self.conns
            .get_mut(name)
            .expect("known player")
            .send_packet(&pkt)
            .await
            .expect("send PlaySound eject");
    }

    /// Build a `PlaySoundPacketAny::V975` for any peer version >= V975 (including
    /// V1001). The `versioned_codec_dispatch!` macro routes `version >= V975` to
    /// `V975Codec`, which correctly encodes only the `V975` variant; constructing
    /// any other variant at that codec would silently fall back to a default-zero
    /// packet. Position follows the Bedrock 1/8-block fixed-point convention: the
    /// wire value is world_coord * 8 (handler divides by 8 to recover block coords).
    fn play_packet(name: &str, x: i32, y: i32, z: i32) -> PlaySoundPacketAny {
        PlaySoundPacketAny::V975(PlaySoundPacketV975 {
            name: name.to_string(),
            position: BlockPos::new(x * 8, y * 8, z * 8),
            volume: 1.0,
            pitch: 1.0,
            server_sound_handle: None,
        })
    }
}
