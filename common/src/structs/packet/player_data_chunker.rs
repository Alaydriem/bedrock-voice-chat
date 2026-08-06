use crate::PlayerEnum;

use super::packet_sender::PacketSender;
use super::packet_type::PacketType;
use super::player_data_packet::PlayerDataPacket;
use super::quic_network_packet::{MAX_DATAGRAM_SIZE, QuicNetworkPacket};
use super::quic_network_packet_data::QuicNetworkPacketData;

// Secondary bound only -- encoded size is the binding constraint. This caps
// per-packet decode and allocation cost if the encoding ever shrinks enough
// that hundreds of players would otherwise fit in a single datagram.
const MAX_PLAYERS_PER_CHUNK: usize = 32;

/// Splits a player roster into groups that each encode within
/// [`MAX_DATAGRAM_SIZE`].
///
/// Player size varies with name length, with which optional identifiers are
/// populated, and with the world identifier format -- a hyphenated UUID from
/// the BDS mod, a 64-character blake3 digest from the client proxy -- so a
/// fixed player count cannot bound the encoded size. It lives beside
/// [`PlayerDataPacket`] so the sizing rules and the wire form cannot drift
/// apart.
pub struct PlayerDataChunker;

impl PlayerDataChunker {
    pub fn chunk(players: Vec<PlayerEnum>, sender: Option<&PacketSender>) -> Vec<Vec<PlayerEnum>> {
        let hoists = PlayerDataPacket::shared_world_uuid(&players).is_some();
        let budget = MAX_DATAGRAM_SIZE.saturating_sub(Self::envelope_overhead(sender, hoists));

        let mut chunks: Vec<Vec<PlayerEnum>> = Vec::new();
        let mut current: Vec<PlayerEnum> = Vec::new();
        let mut current_size = 0usize;

        for player in players {
            let size = Self::encoded_player_size(&player, hoists);

            let over_budget = !current.is_empty() && current_size + size > budget;
            let over_count = current.len() >= MAX_PLAYERS_PER_CHUNK;

            if over_budget || over_count {
                chunks.push(std::mem::take(&mut current));
                current_size = 0;
            }

            current_size += size;
            current.push(player);
        }

        if !current.is_empty() {
            chunks.push(current);
        }

        chunks
            .into_iter()
            .flat_map(|chunk| Self::enforce_datagram_budget(chunk, sender))
            .collect()
    }

    /// Builds the packet this chunker sizes against. Callers send exactly this
    /// so the measurement and the transmitted bytes cannot diverge.
    pub fn packet(players: Vec<PlayerEnum>, sender: Option<&PacketSender>) -> QuicNetworkPacket {
        QuicNetworkPacket {
            sender: sender.cloned(),
            packet_type: PacketType::PlayerData,
            data: QuicNetworkPacketData::PlayerData(PlayerDataPacket::new(players)),
            // Sized and sent by the position broadcaster, not stamped for one connection; the
            // fan-out in `broadcast_to_all` assigns each recipient's sequence.
            ..Default::default()
        }
    }

    // The packing above works from an estimate. This turns the datagram bound
    // into a guarantee by encoding each group for real and halving any that
    // still does not fit, so no arithmetic error above can put an oversized
    // packet on the wire.
    fn enforce_datagram_budget(
        chunk: Vec<PlayerEnum>,
        sender: Option<&PacketSender>,
    ) -> Vec<Vec<PlayerEnum>> {
        if chunk.len() <= 1 || Self::encoded_packet_size(&chunk, sender) <= MAX_DATAGRAM_SIZE {
            return vec![chunk];
        }

        let mut head = chunk;
        let tail = head.split_off(head.len() / 2);

        let mut out = Self::enforce_datagram_budget(head, sender);
        out.extend(Self::enforce_datagram_budget(tail, sender));
        out
    }

    fn encoded_packet_size(players: &[PlayerEnum], sender: Option<&PacketSender>) -> usize {
        postcard::to_stdvec(&Self::packet(players.to_vec(), sender))
            .map(|bytes| bytes.len())
            .unwrap_or(usize::MAX)
    }

    // postcard encodes a sequence as a length prefix followed by the elements
    // back to back, so per-player sizes sum exactly. When the packet hoists the
    // world identifier, players reach the wire without it, so measure the form
    // that will actually be encoded.
    fn encoded_player_size(player: &PlayerEnum, hoists: bool) -> usize {
        let measured = if hoists {
            let mut stripped = player.clone();
            stripped.set_world_uuid(None);
            postcard::to_stdvec(&stripped)
        } else {
            postcard::to_stdvec(player)
        };

        measured
            .map(|bytes| bytes.len())
            .unwrap_or(MAX_DATAGRAM_SIZE)
    }

    // Everything in the datagram that is not a player: packet type, sender, the
    // data variant tag, the hoisted world identifier and the sequence length.
    // Measured against a real empty packet so it tracks changes to those types.
    fn envelope_overhead(sender: Option<&PacketSender>, hoists: bool) -> usize {
        // Room for the sequence length prefix to grow past a single byte.
        const LENGTH_PREFIX_HEADROOM: usize = 8;
        // The largest world identifier in use: a 64-character blake3 digest
        // plus its option tag and length prefix. An empty probe packet carries
        // no world, so it is reserved explicitly.
        const LONGEST_WORLD_UUID: usize = 66;

        let base = postcard::to_stdvec(&Self::packet(Vec::new(), sender))
            .map(|bytes| bytes.len())
            .unwrap_or(0);

        let world = if hoists { LONGEST_WORLD_UUID } else { 0 };

        base + world + LENGTH_PREFIX_HEADROOM
    }
}
