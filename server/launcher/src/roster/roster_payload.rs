use serde_json::{Value, json};

/// The `POST /api/position` body that registers `names` without placing them anywhere.
///
/// Mirrors `common::game_data::MinecraftPlayer`, which the launcher does not link: a death
/// dimension spectator with no world, the same shape `BedrockEventService` broadcasts for a
/// player joining or leaving. Presence expires after the server's 15 s TTL.
pub struct RosterPayload;

impl RosterPayload {
    pub fn body(names: &[String]) -> Value {
        let players: Vec<Value> = names
            .iter()
            .map(|name| {
                json!({
                    "name": name,
                    "coordinates": { "x": 0.0, "y": 0.0, "z": 0.0 },
                    "orientation": { "x": 0.0, "y": 0.0 },
                    "dimension": "death",
                    "deafen": false,
                    "spectator": true
                })
            })
            .collect();
        json!({ "game": "minecraft", "players": players })
    }
}
