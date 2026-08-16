use common::game_data::Dimension;
use common::structs::relay::wire::datagram::VoiceFrame;
use common::traits::player_data::PlayerData;
use common::{Coordinate, MinecraftPlayer, Orientation, PlayerEnum};

// One speaker's audio, flattened for FFI.
//
// The wire carries a `PlayerEnum` because a BVC server routes on it. A bridge
// only ever needs a name, a place and some bytes, and every nested type crossing
// uniffi is another generated class the consumer has to unwrap.
#[derive(Debug, Clone, uniffi::Record)]
pub struct SdkFrame {
    pub speaker: String,
    pub world: Option<String>,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub opus: Vec<u8>,
    pub sample_rate: u32,
    // Signed to match the wire. Narrowing it to unsigned here would put a cast
    // between the server's clock and the bridge's.
    pub timestamp_ms: i64,
    pub spatial: bool,

    // Set when this frame is a jukebox playback rather than speech, carrying the
    // playback it belongs to.
    //
    // Exposed so a bridge can tell the two apart without matching a prefix on the
    // speaker's name. That prefix is a convention inside BVC, not a contract this
    // API makes, and a player named `jukebox-` would defeat it.
    //
    // Concurrent playbacks carry distinct ids, which is what lets a consumer keep
    // them on separate outputs.
    pub jukebox: Option<String>,
}

impl From<VoiceFrame> for SdkFrame {
    fn from(frame: VoiceFrame) -> Self {
        // Read through `PlayerData` rather than matched on the variant. The
        // world identifier is answered by each game for itself, and reaching for
        // a concrete variant to find it is what previously confined peering to
        // one game.
        let speaker = frame.speaker.get_name().to_string();
        let world = frame.speaker.world_identifier().map(str::to_string);
        let position = frame.speaker.get_position().clone();

        Self {
            speaker,
            world,
            x: position.x,
            y: position.y,
            z: position.z,
            opus: frame.opus,
            sample_rate: frame.sample_rate,
            timestamp_ms: frame.timestamp_ms,
            spatial: frame.spatial,
            jukebox: frame.jukebox,
        }
    }
}

impl From<SdkFrame> for VoiceFrame {
    fn from(frame: SdkFrame) -> Self {
        // Outbound is Minecraft because that is what a bridge speaks for: it
        // sits on a Paper server and names players there. This is a statement
        // about the consumer, not a default — a bridge for another game would
        // need the game on the record rather than assumed here.
        Self {
            speaker: PlayerEnum::Minecraft(MinecraftPlayer {
                name: frame.speaker,
                coordinates: Coordinate {
                    x: frame.x,
                    y: frame.y,
                    z: frame.z,
                },
                orientation: Orientation { x: 0.0, y: 0.0 },
                dimension: Dimension::Overworld,
                deafen: false,
                spectator: false,
                world_uuid: None,
                alternative_identity: None,
                player_uuid: None,
                relay_world_uuid: frame.world,
            }),
            sample_rate: frame.sample_rate,
            opus: frame.opus,
            timestamp_ms: frame.timestamp_ms,
            spatial: frame.spatial,
            jukebox: frame.jukebox,
        }
    }
}
