use common::bedrock_protocol::PlayerAuthInputPacket;
use common::bedrock_protocol::StartGamePacket;
use common::bedrock_protocol::protocol::types::generated::PlayerAuthInputData;
use common::bedrock_protocol::protocol::packets::generated::misc::change_dimension::ChangeDimensionPacket;
use common::game_data::Dimension;
use common::players::PlayerEnum;
use common::players::minecraft::MinecraftPlayer;
use common::structs::bedrock::BedrockWorldId;
use common::structs::game::coordinate::Coordinate;
use common::structs::game::orientation::Orientation;
use tauri_plugin_curia::curia;

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
    // Set once this session has decoded a bvc:ctl:sync — proof the BDS world
    // runs the BVC addon (only its panel emits sync). !bvcs: rides are injected
    // only into armed sessions; on a modless server nothing cancels the chat,
    // so an unarmed ride would broadcast the player's audio state publicly.
    bvcs_armed: bool,
}

impl BedrockSessionState {
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
            bvcs_armed: false,
        }
    }

    pub fn arm_bvcs(&mut self) {
        self.bvcs_armed = true;
    }

    pub fn bvcs_armed(&self) -> bool {
        self.bvcs_armed
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
        // Vec2 carries pitch in `x` and yaw in `z`.
        self.orientation = Orientation {
            x: p.player_rotation.x,
            y: p.player_rotation.z,
        };
        let flags = &p.input_data;
        let crawl_start = flags.contains(&PlayerAuthInputData::StartCrawling);
        let crawl_stop = flags.contains(&PlayerAuthInputData::StopCrawling);
        let sneak_flag = flags.contains(&PlayerAuthInputData::Sneaking);
        if crawl_start {
            self.crawling = true;
        }
        if crawl_stop {
            self.crawling = false;
        }
        let prev = self.sneaking;
        self.sneaking = self.crawling || sneak_flag;
        if crawl_start || crawl_stop || prev != self.sneaking {
            curia::debug!("Bedrock state: sneak/crawl changed", {
                input_data: format!("{flags:?}"),
                start_crawl: crawl_start,
                stop_crawl: crawl_stop,
                sneak_flag: sneak_flag,
                crawling: self.crawling,
                sneaking: self.sneaking,
            });
        }
    }

    pub fn apply_change_dimension(&mut self, dimension: i32) {
        self.dimension = Self::dimension_from_i32(dimension);
        curia::debug!("Bedrock state: ChangeDimension", {
            dimension: format!("{:?}", self.dimension),
        });
    }

    pub fn apply_game_type(&mut self, gamemode: i32) {
        let was_spectator = self.spectator;
        self.spectator = Self::is_spectator_gamemode(gamemode);
        if was_spectator != self.spectator {
            curia::debug!("Bedrock state: gamemode changed", {
                gamemode: gamemode,
                spectator: self.spectator,
            });
        }
    }

    pub fn to_player_enum(&self) -> PlayerEnum {
        PlayerEnum::Minecraft(MinecraftPlayer {
            name: self.name.clone(),
            coordinates: self.coordinates.clone(),
            orientation: self.orientation.clone(),
            dimension: self.dimension.clone(),
            whispering: self.sneaking,
            spectator: self.spectator,
            world_uuid: None,
            alternative_identity: None,
            player_uuid: self.player_uuid.clone(),
            relay_world_uuid: self.world_uuid.clone(),
            bridged_voice: false,
        })
    }

    pub fn to_departed_player_enum(&self) -> PlayerEnum {
        PlayerEnum::Minecraft(MinecraftPlayer {
            name: self.name.clone(),
            coordinates: self.coordinates.clone(),
            orientation: self.orientation.clone(),
            dimension: Dimension::Death,
            whispering: self.sneaking,
            spectator: true,
            world_uuid: None,
            alternative_identity: None,
            player_uuid: self.player_uuid.clone(),
            relay_world_uuid: self.world_uuid.clone(),
            bridged_voice: false,
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

    #[cfg(any(test, feature = "e2e"))]
    pub fn set_world_uuid_for_test(&mut self, uuid: String) {
        self.world_uuid = Some(uuid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_player_enum_sets_relay_world_uuid_and_keeps_world_uuid_none() {
        let mut s = BedrockSessionState::new("TestPlayer".to_string(), None);
        s.world_uuid = Some(BedrockWorldId::derive(123, "lvl", "World"));

        let PlayerEnum::Minecraft(player) = s.to_player_enum() else {
            panic!("expected Minecraft variant");
        };
        assert_eq!(player.world_uuid, None);
        assert_eq!(player.relay_world_uuid, s.world_uuid);
    }

    #[test]
    fn to_departed_player_enum_sets_relay_world_uuid_and_keeps_world_uuid_none() {
        let mut s = BedrockSessionState::new("TestPlayer".to_string(), None);
        s.world_uuid = Some(BedrockWorldId::derive(123, "lvl", "World"));

        let PlayerEnum::Minecraft(player) = s.to_departed_player_enum() else {
            panic!("expected Minecraft variant");
        };
        assert_eq!(player.world_uuid, None);
        assert_eq!(player.relay_world_uuid, s.world_uuid);
    }
}
