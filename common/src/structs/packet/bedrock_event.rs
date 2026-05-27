use serde::{Deserialize, Serialize};

use crate::game_data::minecraft::Dimension;
use crate::structs::game::Coordinate;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum BedrockEvent {
    JukeboxInsert {
        audio_id: String,
        block_pos: Coordinate,
        dimension: Dimension,
        player_xuid: String,
    },
    JukeboxEject {
        event_id: String,
        player_xuid: String,
    },
    PlayerDeath {
        player_xuid: String,
        dimension: Dimension,
        last_pos: Coordinate,
    },
    PlayerLeave {
        player_xuid: String,
    },
    JukeboxEjectAnnouncement {
        event_id: String,
        block_pos: Coordinate,
    },
}
