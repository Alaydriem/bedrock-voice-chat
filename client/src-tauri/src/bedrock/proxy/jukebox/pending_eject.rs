use common::structs::game::Coordinate;

pub struct PendingEject {
    pub event_id: String,
    pub world_uuid: String,
    pub block_pos: Coordinate,
}
