use common::structs::game::Coordinate;

pub(crate) enum JukeboxCommand {
    Play {
        audio_id: String,
        pos: Coordinate,
        dimension: String,
    },
    Eject {
        pos: Coordinate,
        dimension: String,
    },
}
