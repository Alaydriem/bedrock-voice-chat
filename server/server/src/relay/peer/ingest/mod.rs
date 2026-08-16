pub mod rejection;

pub use rejection::IngestRejection;

use std::sync::Arc;

use common::structs::packet::{
    AudioFrameMetadata, AudioFramePacket, JukeboxMetadata, PacketSender, PacketType,
    QuicNetworkPacket, QuicNetworkPacketData,
};
use common::structs::relay::wire::datagram::VoiceFrame;
use common::traits::player_data::PlayerData;
use iroh::PublicKey;

use crate::relay::grant::GrantTable;

use super::local_clients::LocalClients;

// The peer boundary.
//
// Everything a peer sends passes through here, and what comes out is a packet
// this server built rather than one it forwarded. The wire carries no
// `PacketSender`, so there is no inbound identity to trust — the design's rule
// about stamping rather than trusting is structural here rather than a check.
pub struct PeerIngest {
    grants: Arc<GrantTable>,
    locals: Arc<dyn LocalClients>,
}

impl PeerIngest {
    pub fn new(grants: Arc<GrantTable>, locals: Arc<dyn LocalClients>) -> Self {
        Self { grants, locals }
    }

    pub fn admit(
        &self,
        node: &PublicKey,
        frame: VoiceFrame,
    ) -> Result<QuicNetworkPacket, IngestRejection> {
        let speaker = frame.speaker.get_name().to_string();

        let world = frame
            .speaker
            .world_identifier()
            .ok_or(IngestRejection::NoWorld)?
            .to_string();

        if !self.grants.may_carry(node, &world) {
            return Err(IngestRejection::NotGranted { speaker, world });
        }

        // A peer naming one of our own players could overwrite that player's
        // cached position and inherit their channel membership, and channel
        // membership bypasses the proximity gate.
        let identity = frame.speaker.get_game().membership_key(&speaker);
        if self.locals.has_live_client(&identity) {
            return Err(IngestRejection::ImpersonatesLocalPlayer { speaker });
        }

        // Rebuilt from the speaker rather than carried across. The position and
        // dimension a beacon needs are already on it, and sending them a second
        // time only invites the two copies to disagree.
        //
        // Peers share a relay world, which is to say they serve the same world, so
        // the block this marks is one the receiving side's players can reach.
        let jukebox = frame.jukebox.as_ref().and_then(|event_id| {
            Some(AudioFrameMetadata::Jukebox(JukeboxMetadata::new(
                frame.speaker.get_position().clone(),
                event_id.clone(),
                frame.speaker.dimension()?,
            )))
        });

        // The timestamp is taken locally rather than carried across: it feeds the
        // receiving side's jitter buffer, and two servers do not share a clock.
        let mut audio = AudioFramePacket::new(
            frame.opus,
            frame.sample_rate,
            Some(frame.speaker),
            Some(frame.spatial),
        );

        if let Some(jukebox) = jukebox {
            audio = audio.with_metadata(vec![jukebox]);
        }

        Ok(QuicNetworkPacket {
            packet_type: PacketType::AudioFrame,
            data: QuicNetworkPacketData::AudioFrame(audio),
            sender: Some(PacketSender::synthetic(identity)),
            ..Default::default()
        })
    }
}
