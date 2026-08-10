use common::PlayerEnum;
use common::structs::position::{PresenceKind, RelativePosition};
use common::traits::player_data::PlayerData;

/// How far past voice range the feed reaches.
///
/// Derived rather than configured: a fixed constant could fall inside a
/// server's configured `broadcast_range`, which would defeat the feature --
/// entries would only appear once the player was already audible.
pub const POSITION_SCOPE_MULTIPLIER: f32 = 2.5;

/// Hard ceiling on scope.
///
/// Scope drives a per-observer scan of the world on every tick, so an
/// unbounded value is a server-side denial of service.
pub const POSITION_SCOPE_MAX: f32 = 256.0;

/// How many entries beyond voice range one snapshot carries.
///
/// Everything inside voice range is sent uncapped, because that tier is the roster and its
/// size is already bounded by the audio the server is mixing for this client. Beyond it only
/// a handful are ever drawn, so a crowded square two hundred blocks away must not become
/// kilobytes of JSON twice a second.
pub const FAR_TIER_MAX: usize = 16;

/// Derives an observer's view of the players around them.
///
/// Scope is decided by [`PlayerEnum::can_communicate_with`], the same per-game
/// rule voice routing uses -- relay world, world, dimension, spectator state
/// and distance -- but at feed scope rather than voice range. Reusing it means
/// a new game's rules apply here the moment they apply to audio.
pub struct PositionService {
    voice_range: f32,
    scope_range: f32,
}

impl PositionService {
    pub fn for_voice_range(voice_range: f32) -> Self {
        Self {
            voice_range,
            scope_range: (voice_range * POSITION_SCOPE_MULTIPLIER).clamp(0.0, POSITION_SCOPE_MAX),
        }
    }

    pub fn scope_range(&self) -> f32 {
        self.scope_range
    }

    pub fn voice_range(&self) -> f32 {
        self.voice_range
    }

    /// Two tiers in one snapshot, nearest first.
    ///
    /// Sorting by distance is what makes the far tier's cap safe: it can only ever drop
    /// somebody already beyond voice range, never a player whose card is waiting on a
    /// distance.
    ///
    /// `is_on_voice` answers by canonical identity, which is how the QUIC registry names
    /// connections.
    pub fn snapshot_positions(
        &self,
        observer: &PlayerEnum,
        world: &[PlayerEnum],
        is_on_voice: &dyn Fn(&str) -> bool,
    ) -> Vec<RelativePosition> {
        let observer_name = observer.get_name();
        let mut near: Vec<RelativePosition> = Vec::new();
        let mut far: Vec<RelativePosition> = Vec::new();

        for candidate in world {
            if candidate.get_name() == observer_name {
                continue;
            }
            if candidate
                .can_communicate_with(observer, self.scope_range)
                .is_err()
            {
                continue;
            }

            let entry = self.relative(observer, candidate, is_on_voice);
            if f32::from(entry.distance) <= self.voice_range {
                near.push(entry);
            } else {
                far.push(entry);
            }
        }

        near.sort_by_key(|entry| entry.distance);
        far.sort_by_key(|entry| entry.distance);
        far.truncate(FAR_TIER_MAX);
        near.extend(far);
        near
    }

    fn relative(
        &self,
        observer: &PlayerEnum,
        candidate: &PlayerEnum,
        is_on_voice: &dyn Fn(&str) -> bool,
    ) -> RelativePosition {
        let from = observer.get_position();
        let to = candidate.get_position();

        let dx = to.x - from.x;
        let dz = to.z - from.z;
        let dy = to.y - from.y;

        let distance = (dx * dx + dz * dz).sqrt();

        // Minecraft yaw is degrees clockwise from south, while atan2 sweeps the other
        // way; negating dx puts the target in yaw's own frame, so subtracting yaw leaves
        // degrees clockwise from wherever the observer is facing and the client can draw
        // the entry without ever seeing a coordinate.
        let absolute = (-dx).atan2(dz).to_degrees();
        let bearing = (absolute - observer.get_orientation().y).rem_euclid(360.0);

        let identity = candidate.identity();

        RelativePosition {
            presence: if is_on_voice(&identity) {
                PresenceKind::Voice
            } else {
                PresenceKind::Game
            },
            name: identity,
            bearing_deg: bearing.round() as u16 % 360,
            distance: distance.round().clamp(0.0, f32::from(u16::MAX)) as u16,
            elevation: dy.round().clamp(-32768.0, 32767.0) as i16,
        }
    }
}
