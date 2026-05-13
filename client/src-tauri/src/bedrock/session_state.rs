use common::bedrock_protocol::{
    ChangeDimensionPacket, PlayerAuthInputPacket, StartGamePacket,
};
use common::game_data::Dimension;
use common::players::minecraft::MinecraftPlayer;
use common::players::PlayerEnum;
use common::structs::bedrock::BedrockWorldId;
use common::structs::game::coordinate::Coordinate;
use common::structs::game::orientation::Orientation;
use log::debug;

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
        debug!(
            "Bedrock state: StartGame gamemode={} spectator={} dimension={:?}",
            p.player_gamemode, self.spectator, self.dimension
        );
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
        let was_sneaking = self.sneaking;
        let was_crawling = self.crawling;
        self.sneaking = flags & Self::SNEAKING_BIT != 0;
        if flags & Self::START_CRAWLING_BIT != 0 {
            self.crawling = true;
        }
        if flags & Self::STOP_CRAWLING_BIT != 0 {
            self.crawling = false;
        }
        if was_sneaking != self.sneaking || was_crawling != self.crawling {
            debug!(
                "Bedrock state: input flags=0x{:x} sneaking={} crawling={} deafen={}",
                flags,
                self.sneaking,
                self.crawling,
                self.sneaking || self.crawling
            );
        }
    }

    pub fn apply_change_dimension(&mut self, p: &ChangeDimensionPacket) {
        self.dimension = Self::dimension_from_i32(p.dimension);
        debug!("Bedrock state: ChangeDimension -> {:?}", self.dimension);
    }

    pub fn apply_game_type(&mut self, gamemode: i32) {
        let was_spectator = self.spectator;
        self.spectator = Self::is_spectator_gamemode(gamemode);
        if was_spectator != self.spectator {
            debug!(
                "Bedrock state: gamemode={} spectator={}",
                gamemode, self.spectator
            );
        }
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

    pub fn world_uuid(&self) -> Option<&str> {
        self.world_uuid.as_deref()
    }

    pub fn dimension(&self) -> Dimension {
        self.dimension.clone()
    }

    pub fn coordinates(&self) -> Coordinate {
        self.coordinates.clone()
    }

    pub fn player_uuid(&self) -> Option<&str> {
        self.player_uuid.as_deref()
    }

    pub fn name(&self) -> &str {
        &self.name
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
        matches!(gamemode, 3 | 4 | 6)
    }
}
