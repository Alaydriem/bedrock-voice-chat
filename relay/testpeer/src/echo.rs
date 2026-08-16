use std::time::Duration;

use bvc_relay::node::NodeIdentity;
use bvc_relay::peer::{Handshake, PeerAuthority, PeerEndpoint, PeerLink, PeerScope};
use common::game_data::Dimension;
use common::structs::relay::Capability;
use common::structs::relay::wire::datagram::VoiceFrame;
use common::{Coordinate, MinecraftPlayer, Orientation, PlayerEnum};

// A peer that accepts anyone and talks.
//
// It exists so a binding test has something real to dial: a mock would prove the
// generated Kotlin compiles, and what needs proving is that it carries a frame
// off an actual wire and releases a reader parked on one.
pub struct EchoPeer;

struct AcceptsAnyone;

impl PeerAuthority for AcceptsAnyone {
    fn authorize(&self, _node: &iroh::PublicKey, declared: &[String]) -> Option<PeerScope> {
        Some(PeerScope {
            worlds: declared.to_vec(),
            capabilities: vec![Capability::CarrySpeakers],
        })
    }
}

impl EchoPeer {
    const TICK: Duration = Duration::from_millis(100);

    // `burst` bounds how many frames each link receives before the peer goes
    // quiet. A test asserting that a parked read is released cannot use a peer
    // that never stops talking — it would be asserting which of the two won a
    // race. `None` sends continuously.
    //
    // `jukebox` stamps every frame as a playback, so a binding test can check the
    // field arrives rather than trusting that it was encoded.
    pub async fn run(
        burst: Option<usize>,
        jukebox: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let identity = NodeIdentity::load_or_create(dir.path().to_str().ok_or("path")?)?;
        let endpoint = PeerEndpoint::bind(&identity, None).await?;

        // The only line the harness parses. Flushed immediately, because the
        // caller is blocked reading for it.
        println!("PEERLINK={}", endpoint.ticket().await?);
        use std::io::Write;
        std::io::stdout().flush()?;

        while let Some(incoming) = endpoint.endpoint().accept().await {
            let jukebox = jukebox.clone();
            tokio::spawn(async move {
                let Ok(conn) = incoming.await else { return };
                let Ok(accept) = Handshake::accept(&conn, &AcceptsAnyone).await else {
                    return;
                };
                let Ok(link) = PeerLink::establish(conn, accept.worlds) else {
                    return;
                };

                Self::talk(link, burst, jukebox).await;
            });
        }

        Ok(())
    }

    async fn talk(link: PeerLink, burst: Option<usize>, jukebox: Option<String>) {
        let mut ticker = tokio::time::interval(Self::TICK);
        let mut sent: usize = 0;

        loop {
            if burst.is_some_and(|limit| sent >= limit) {
                break;
            }

            ticker.tick().await;
            sent += 1;

            if link.send(Self::frame(sent as u8, jukebox.clone())).is_err() {
                return;
            }
        }

        // Held open and quiet. Dropping the link closes the connection, which the
        // far side reads as a disconnect and answers with a redial.
        while link.recv().await.is_ok() {}
    }

    fn frame(marker: u8, jukebox: Option<String>) -> VoiceFrame {
        VoiceFrame {
            speaker: PlayerEnum::Minecraft(MinecraftPlayer {
                name: "EchoPeer".to_string(),
                coordinates: Coordinate {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                },
                orientation: Orientation { x: 0.0, y: 0.0 },
                dimension: Dimension::Overworld,
                deafen: false,
                spectator: false,
                world_uuid: None,
                alternative_identity: None,
                player_uuid: None,
                relay_world_uuid: Some("W1".to_string()),
            }),
            sample_rate: 48000,
            opus: vec![marker],
            timestamp_ms: 0,
            spatial: true,
            jukebox,
        }
    }
}
