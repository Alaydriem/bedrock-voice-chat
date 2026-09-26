use common::game_data::Dimension;
use common::players::MinecraftPlayer;
use common::structs::packet::SpeakerPosition;
use common::{Game, Orientation, PlayerEnum};

/// Builds the player a recording header holds, from what an audio frame carries.
///
/// The header has always held a whole player and the renderer reads a position and a whispering
/// flag back out of it, so the shape stays and the value is composed here. That is what keeps
/// `RECORDING_VERSION` still and sessions recorded before this change exportable.
pub struct RecordedPlayer;

impl RecordedPlayer {
    /// `MinecraftPlayer` and not `GenericPlayer`: `GenericPlayer` has no whispering field, so
    /// `is_whispering()` would fall through to the trait default of `false` and a whispering
    /// speaker would be recorded as heard at full range.
    ///
    /// The dimension and the zero orientation are synthesised, not claimed. Nothing reads
    /// either off the emitter — the renderer takes orientation from the listener — and the
    /// alternative is widening the recording format.
    pub fn synthesise(name: &str, speaker: &SpeakerPosition) -> PlayerEnum {
        PlayerEnum::Minecraft(MinecraftPlayer {
            name: Game::display_name(name).to_string(),
            coordinates: speaker.position.clone(),
            orientation: Orientation { x: 0.0, y: 0.0 },
            dimension: Dimension::Overworld,
            whispering: speaker.whispering,
            spectator: false,
            world_uuid: None,
            alternative_identity: None,
            player_uuid: None,
            relay_world_uuid: None,
            bridged_voice: false,
        })
    }
}
