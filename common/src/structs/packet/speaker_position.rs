use serde::{Deserialize, Serialize};

use crate::traits::player_data::PlayerData;
use crate::{Coordinate, PlayerEnum};

/// Where a speaker is, as a listener needs it.
///
/// Everything a `PlayerEnum` carries beyond these two facts is the server's business: the
/// world and relay identifiers scope peering, the dimension and spectator flags gate routing,
/// the orientation belongs to whoever is listening, and the name is already on the envelope.
/// A listener reads a position and whether the speaker is deafened, and nothing else.
///
/// `Clone` rather than `Copy`, because `Coordinate` is not `Copy`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SpeakerPosition {
    pub position: Coordinate,
    /// A deafened speaker plays centre-panned at unity. The server has already applied
    /// `deafen_distance`, so the frame arriving is the decision and the client skips
    /// attenuation entirely.
    pub deafened: bool,
}

impl SpeakerPosition {
    pub fn new(position: Coordinate, deafened: bool) -> Self {
        Self { position, deafened }
    }

    /// Reduces a player to what a listener reads.
    ///
    /// Taken from the player rather than from a caller's arguments, so the position on the
    /// frame and the position the server routed from cannot disagree.
    pub fn from_player(player: &PlayerEnum) -> Self {
        Self {
            position: player.get_position().clone(),
            deafened: player.is_deafened(),
        }
    }
}
