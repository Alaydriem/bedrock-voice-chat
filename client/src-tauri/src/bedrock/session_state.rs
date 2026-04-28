use common::bedrock_protocol::{
    ChangeDimensionPacket, PlayerAuthInputPacket, StartGamePacket,
};
use common::game_data::Dimension;
use common::players::minecraft::MinecraftPlayer;
use common::players::PlayerEnum;
use common::structs::bedrock::BedrockWorldId;
use common::structs::game::coordinate::Coordinate;
use common::structs::game::orientation::Orientation;

pub struct BedrockSessionState {
    name: String,
    player_uuid: Option<String>,
    world_uuid: Option<String>,
    coordinates: Coordinate,
    orientation: Orientation,
    dimension: Dimension,
    sneaking: bool,
    crawling: bool,
    spectator: bool,
}

impl BedrockSessionState {
    const SNEAKING_BIT: u128 = 1u128 << 8;
    const START_CRAWLING_BIT: u128 = 1u128 << 40;
    const STOP_CRAWLING_BIT: u128 = 1u128 << 41;

    pub fn new(name: String, player_uuid: Option<String>) -> Self {
        Self {
            name,
            player_uuid,
            world_uuid: None,
            coordinates: Coordinate::default(),
            orientation: Orientation { x: 0.0, y: 0.0 },
            dimension: Dimension::default(),
            sneaking: false,
            crawling: false,
            spectator: false,
        }
    }

    pub fn apply_start_game(&mut self, p: &StartGamePacket) {
        self.dimension = Self::dimension_from_i32(p.dimension);
        self.spectator = Self::is_spectator_gamemode(p.player_gamemode);
        self.world_uuid = Some(BedrockWorldId::derive(p.seed, &p.level_id, &p.world_name));
    }

    pub fn apply_position(&mut self, p: &PlayerAuthInputPacket) {
        self.coordinates = Coordinate {
            x: p.position.x,
            y: p.position.y,
            z: p.position.z,
        };
        self.orientation = Orientation {
            x: p.yaw,
            y: p.pitch,
        };
        let flags = p.input_data.0;
        self.sneaking = flags & Self::SNEAKING_BIT != 0;
        if flags & Self::START_CRAWLING_BIT != 0 {
            self.crawling = true;
        }
        if flags & Self::STOP_CRAWLING_BIT != 0 {
            self.crawling = false;
        }
    }

    pub fn apply_change_dimension(&mut self, p: &ChangeDimensionPacket) {
        self.dimension = Self::dimension_from_i32(p.dimension);
    }

    pub fn apply_game_type(&mut self, gamemode: i32) {
        self.spectator = Self::is_spectator_gamemode(gamemode);
    }

    pub fn to_player_enum(&self) -> PlayerEnum {
        PlayerEnum::Minecraft(MinecraftPlayer {
            name: self.name.clone(),
            coordinates: self.coordinates.clone(),
            orientation: self.orientation.clone(),
            dimension: self.dimension.clone(),
            deafen: self.sneaking || self.crawling,
            spectator: self.spectator,
            world_uuid: self.world_uuid.clone(),
            alternative_identity: None,
            player_uuid: self.player_uuid.clone(),
        })
    }

    fn dimension_from_i32(d: i32) -> Dimension {
        match d {
            0 => Dimension::Overworld,
            1 => Dimension::TheNether,
            2 => Dimension::TheEnd,
            _ => Dimension::Overworld,
        }
    }

    fn is_spectator_gamemode(gamemode: i32) -> bool {
        matches!(gamemode, 3 | 4)
    }
}
