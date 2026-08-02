mod handle;

pub use handle::PositionHandle;

use common::PlayerEnum;
use common::structs::position::RelativePosition;
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

/// Derives an observer's anonymised view of the players around them.
///
/// Scope is decided by [`PlayerEnum::can_communicate_with`], the same per-game
/// rule voice routing uses -- relay world, world, dimension, spectator state
/// and distance -- but at feed scope rather than voice range. Reusing it means
/// a new game's rules apply here the moment they apply to audio.
pub struct PositionService {
    scope_range: f32,
}

impl PositionService {
    pub fn for_voice_range(voice_range: f32) -> Self {
        Self {
            scope_range: (voice_range * POSITION_SCOPE_MULTIPLIER).clamp(0.0, POSITION_SCOPE_MAX),
        }
    }

    pub fn scope_range(&self) -> f32 {
        self.scope_range
    }

    pub fn snapshot_positions(
        &self,
        observer: &PlayerEnum,
        world: &[PlayerEnum],
        handles: &PositionHandle,
    ) -> Vec<RelativePosition> {
        let observer_name = observer.get_name();

        world
            .iter()
            .filter(|candidate| candidate.get_name() != observer_name)
            .filter(|candidate| {
                candidate
                    .can_communicate_with(observer, self.scope_range)
                    .is_ok()
            })
            .map(|candidate| self.relative(observer, candidate, handles))
            .collect()
    }

    fn relative(
        &self,
        observer: &PlayerEnum,
        candidate: &PlayerEnum,
        handles: &PositionHandle,
    ) -> RelativePosition {
        let from = observer.get_position();
        let to = candidate.get_position();

        let dx = to.x - from.x;
        let dz = to.z - from.z;
        let dy = to.y - from.y;

        let distance = (dx * dx + dz * dz).sqrt();

        // Minecraft yaw is degrees clockwise from south; subtracting it turns an
        // absolute bearing into one relative to where the observer is facing, so
        // the client can draw the entry without ever seeing a coordinate.
        let absolute = dx.atan2(dz).to_degrees();
        let bearing = (absolute - observer.get_orientation().y).rem_euclid(360.0);

        RelativePosition {
            handle: handles.handle_for(candidate.get_name()),
            bearing_deg: bearing.round() as u16 % 360,
            distance: distance.round().clamp(0.0, f32::from(u16::MAX)) as u16,
            elevation: dy.round().clamp(-32768.0, 32767.0) as i16,
        }
    }
}
