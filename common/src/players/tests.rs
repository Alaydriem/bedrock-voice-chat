#[cfg(test)]
mod tests {
    use super::super::{MinecraftPlayer, PlayerEnum, GenericPlayer};
    use crate::{Coordinate, Orientation, Game};
    use crate::game_data::{Dimension, GameDataCollection};

    #[test]
    fn test_player_enum_postcard_serialization() {
        // Create a MinecraftPlayer
        let minecraft_player = MinecraftPlayer {
            name: "TestPlayer".to_string(),
            coordinates: Coordinate { x: 1.0, y: 2.0, z: 3.0 },
            orientation: Orientation { x: 0.0, y: 90.0 },
            dimension: Dimension::Overworld,
            deafen: false,
            spectator: false,
        };

        let player_enum = PlayerEnum::Minecraft(minecraft_player.clone());

        // Serialize with postcard
        let serialized = postcard::to_allocvec(&player_enum)
            .expect("Failed to serialize with postcard");

        println!("Serialized {} bytes", serialized.len());

        // Deserialize with postcard
        let deserialized: PlayerEnum = postcard::from_bytes(&serialized)
            .expect("Failed to deserialize with postcard");

        // Verify the data
        match deserialized {
            PlayerEnum::Minecraft(p) => {
                assert_eq!(p.name, "TestPlayer");
                assert_eq!(p.coordinates.x, 1.0);
                assert_eq!(p.dimension, Dimension::Overworld);
            }
            _ => panic!("Expected Minecraft player"),
        }
    }

    #[test]
    fn test_generic_player_postcard_serialization() {
        let generic_player = GenericPlayer {
            name: "HytalePlayer".to_string(),
            coordinates: Coordinate { x: 5.0, y: 10.0, z: 15.0 },
            orientation: Orientation { x: 45.0, y: 180.0 },
            game: Game::Hytale,
        };

        let player_enum = PlayerEnum::Generic(generic_player);

        // Serialize with postcard
        let serialized = postcard::to_allocvec(&player_enum)
            .expect("Failed to serialize generic player");

        // Deserialize with postcard
        let deserialized: PlayerEnum = postcard::from_bytes(&serialized)
            .expect("Failed to deserialize generic player");

        // Verify
        match deserialized {
            PlayerEnum::Generic(p) => {
                assert_eq!(p.name, "HytalePlayer");
                assert_eq!(p.game, Game::Hytale);
            }
            _ => panic!("Expected Generic player"),
        }
    }

    #[test]
    fn test_game_data_collection_json_deserialization() {
        // Test JSON format from legacy Minecraft mod
        let json = r#"{
  "game": "minecraft",
  "players": [
    {
      "name": "Alaydriem",
      "dimension": "overworld",
      "coordinates": {
        "x": 336.0,
        "y": 78.0,
        "z": -690.0
      },
      "orientation": {
        "x": 0,
        "y": 120
      },
      "deafen": false
    }
  ]
}"#;

        let collection: GameDataCollection = serde_json::from_str(json)
            .expect("Failed to deserialize GameDataCollection from JSON");

        assert_eq!(collection.game, Some(Game::Minecraft));
        assert_eq!(collection.players.len(), 1);

        match &collection.players[0] {
            PlayerEnum::Minecraft(p) => {
                assert_eq!(p.name, "Alaydriem");
                assert_eq!(p.coordinates.x, 336.0);
                assert_eq!(p.dimension, Dimension::Overworld);
                assert_eq!(p.deafen, false);
            }
            _ => panic!("Expected Minecraft player"),
        }
    }
}
