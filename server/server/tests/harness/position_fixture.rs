use common::PlayerEnum;
use common::game_data::Dimension;
use common::players::MinecraftPlayer;
use common::{Coordinate, Orientation};

/// The world identifier formats in production. They differ in encoded length,
/// which is what makes a fixed per-player size estimate unsafe.
#[derive(Debug, Clone, Copy)]
pub enum WorldForm {
    /// Hyphenated UUIDv4, as the BDS mod publishes.
    Uuid,
    /// 64-character blake3 digest, as `BedrockWorldId::derive` produces for the
    /// client proxy.
    Blake3,
    /// A roster with no world identity at all.
    None,
}

const UUID_WORLD: &str = "8f14e45f-ea8f-4b62-9f2a-1c0d7e3b4a55";
const BLAKE3_WORLD: &str = "cba824f1284f220f787dc7f56a42fa03fb40f885c43412a2b7287f85238c882d";
const OTHER_UUID_WORLD: &str = "1b9d6bcd-bbfd-4b2d-9b5d-ab8dfbbd4bed";

/// Builds randomized player rosters for position-broadcast tests.
///
/// Randomness is drawn from a fixed seed so a failure reproduces exactly; the
/// point is to vary the fields that drive encoded size -- name length and which
/// optional identifiers are populated -- not to be unpredictable run to run.
pub struct PositionFixture;

impl PositionFixture {
    pub const WORLD_FORMS: [WorldForm; 3] = [WorldForm::Uuid, WorldForm::Blake3, WorldForm::None];

    pub fn roster(count: usize, world: WorldForm) -> Vec<PlayerEnum> {
        let world_uuid = match world {
            WorldForm::Uuid => Some(UUID_WORLD),
            WorldForm::Blake3 => Some(BLAKE3_WORLD),
            WorldForm::None => None,
        };

        (0..count)
            .map(|i| Self::player(i, world_uuid))
            .collect()
    }

    /// A roster split across two worlds, which suppresses the compact wire form.
    pub fn mixed_world_roster(count: usize) -> Vec<PlayerEnum> {
        (0..count)
            .map(|i| {
                let world = if i % 2 == 0 {
                    UUID_WORLD
                } else {
                    OTHER_UUID_WORLD
                };
                Self::player(i, Some(world))
            })
            .collect()
    }

    /// The payload the BDS mod POSTs to `/api/position`, field for field as
    /// `Payload.toJSON` emits it. Deserializing this is the contract the
    /// positions route binds, so a roster that survives it is the same roster
    /// the broadcast path receives in production.
    pub fn mod_payload_json(count: usize, world: WorldForm) -> String {
        let players: Vec<String> = Self::roster(count, world)
            .iter()
            .map(|player| {
                let PlayerEnum::Minecraft(p) = player else {
                    unreachable!("roster builds Minecraft players")
                };

                let world_uuid = p
                    .world_uuid
                    .as_ref()
                    .map(|w| format!(r#","world_uuid":"{w}""#))
                    .unwrap_or_default();

                format!(
                    r#"{{"name":"{}","dimension":"overworld","coordinates":{{"x":{},"y":{},"z":{}}},"deafen":{},"orientation":{{"x":{},"y":{}}},"spectator":{}{}}}"#,
                    p.name,
                    p.coordinates.x,
                    p.coordinates.y,
                    p.coordinates.z,
                    p.deafen,
                    p.orientation.x,
                    p.orientation.y,
                    p.spectator,
                    world_uuid,
                )
            })
            .collect();

        format!(
            r#"{{"game":"minecraft","players":[{}]}}"#,
            players.join(",")
        )
    }

    fn player(index: usize, world_uuid: Option<&str>) -> PlayerEnum {
        let mut rng = Lcg::seeded(index as u64);

        PlayerEnum::Minecraft(MinecraftPlayer {
            name: Self::gamertag(index, &mut rng),
            coordinates: Coordinate {
                x: rng.coordinate(),
                y: rng.coordinate(),
                z: rng.coordinate(),
            },
            orientation: Orientation {
                x: rng.coordinate(),
                y: rng.coordinate(),
            },
            dimension: Dimension::Overworld,
            deafen: rng.next() % 5 == 0,
            spectator: rng.next() % 7 == 0,
            world_uuid: world_uuid.map(String::from),
            alternative_identity: None,
            // Populated for a slice of the roster: a Floodgate/Java player
            // carries one, and it is the second-largest field per player.
            player_uuid: (rng.next() % 3 == 0)
                .then(|| format!("3c9a1b77-2d44-4e18-8b6c-{:012x}", index)),
            relay_world_uuid: None,
        })
    }

    // Gamertags run 3-16 characters; length drives per-player encoded size, so
    // the roster spans the whole range. The index keeps names unique, which the
    // delivery assertions rely on.
    fn gamertag(index: usize, rng: &mut Lcg) -> String {
        const MIN: usize = 3;
        const MAX: usize = 16;

        let unique = format!("P{index}");
        let target = MIN + (rng.next() as usize % (MAX - MIN + 1));

        let mut name = unique;
        while name.len() < target {
            name.push('x');
        }
        name
    }
}

// A tiny deterministic generator; the tests need reproducible variation, not
// statistical quality, and this avoids a dev-dependency for it.
struct Lcg(u64);

impl Lcg {
    fn seeded(seed: u64) -> Self {
        Self(seed.wrapping_mul(6364136223846793005).wrapping_add(1))
    }

    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }

    fn coordinate(&mut self) -> f32 {
        (self.next() % 60_000_000) as f32 / 1000.0 - 30_000.0
    }
}
