use std::collections::{HashMap, HashSet};

use common::PlayerEnum;
use common::traits::player_data::PlayerData;

use super::GridCell;

/// One tick's picture of the world, bucketed so an observer's neighbours are a lookup.
///
/// Built once per tick and shared by every open socket. The scan it replaces ran per socket
/// over the whole player cache, which is quadratic in players times observers — the kind of
/// cost that does not degrade, it falls over.
///
/// What it cannot share is the answer. Bearing is relative to the observer's facing and
/// distance is from their position, so two players in the same cell get different numbers
/// for the same third party. Sending world-relative bearings and rotating client-side fails
/// on the second step, because distance would have to move client-side too and that means
/// sending absolute coordinates — the one thing this feed exists not to do.
pub struct WorldIndex {
    cells: HashMap<GridCell, Vec<PlayerEnum>>,
    by_identity: HashMap<String, PlayerEnum>,
    on_voice: HashSet<String>,
    cell_size: f32,
}

impl WorldIndex {
    /// `on_voice` carries the connections this server terminates itself. Players a mod
    /// declares bridged are added here, because their connection is held by that mod and the
    /// registry cannot see it — leaving them out reports somebody as being in the world with
    /// no voice while their audio is arriving over the peer link.
    pub fn build(world: Vec<PlayerEnum>, on_voice: HashSet<String>, cell_size: f32) -> Self {
        let mut cells: HashMap<GridCell, Vec<PlayerEnum>> = HashMap::new();
        let mut by_identity = HashMap::with_capacity(world.len());
        let mut on_voice = on_voice;

        for player in world {
            // The reserved slot is not a player: it has no position to bucket and no identity
            // to key on, so indexing it would put an entry under the empty identity that
            // nothing looks up and everything near the origin collides with.
            if player.is_reserved() {
                continue;
            }

            if player.has_bridged_voice() {
                on_voice.insert(player.identity().to_string());
            }

            let cell = GridCell::of(player.get_position(), cell_size);
            // Keyed on the canonical identity, the same key `on_voice` uses. Keyed on the bare
            // in-game name, one game's player shadowed the other's in a world that hosts both,
            // and this struct answered two questions about two different people.
            by_identity.insert(player.identity().to_string(), player.clone());
            cells.entry(cell).or_default().push(player);
        }

        Self {
            cells,
            by_identity,
            on_voice,
            cell_size,
        }
    }

    pub fn empty(cell_size: f32) -> Self {
        Self {
            cells: HashMap::new(),
            by_identity: HashMap::new(),
            on_voice: HashSet::new(),
            cell_size,
        }
    }

    /// The observer as the world last reported them, or `None` when they are authenticated
    /// but not in the game yet — a normal state rather than an error.
    ///
    /// `identity` is the canonical `game:gamertag`. A bare gamertag matches nobody, and the
    /// miss is indistinguishable from not being in the game: an empty feed, forever.
    pub fn observer(&self, identity: &str) -> Option<&PlayerEnum> {
        self.by_identity.get(identity)
    }

    /// Everyone close enough to be worth testing against this observer.
    ///
    /// Candidates, not answers: the per-game rule still decides who is actually in scope,
    /// and it is the only thing that knows about worlds, dimensions and spectators.
    pub fn neighbours(&self, observer: &PlayerEnum) -> Vec<PlayerEnum> {
        let home = GridCell::of(observer.get_position(), self.cell_size);
        let mut out = Vec::new();
        for cell in home.ring() {
            if let Some(players) = self.cells.get(&cell) {
                out.extend(players.iter().cloned());
            }
        }
        out
    }

    /// Whether this canonical identity holds a voice connection.
    ///
    /// The registry names connections `game:gamertag`, so a bare gamertag never matches and
    /// every player would read as game-only presence.
    pub fn is_on_voice(&self, identity: &str) -> bool {
        self.on_voice.contains(identity)
    }

    pub fn len(&self) -> usize {
        self.by_identity.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_identity.is_empty()
    }
}
