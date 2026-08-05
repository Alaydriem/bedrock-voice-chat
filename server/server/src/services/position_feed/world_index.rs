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
    by_name: HashMap<String, PlayerEnum>,
    on_voice: HashSet<String>,
    cell_size: f32,
}

impl WorldIndex {
    pub fn build(world: Vec<PlayerEnum>, on_voice: HashSet<String>, cell_size: f32) -> Self {
        let mut cells: HashMap<GridCell, Vec<PlayerEnum>> = HashMap::new();
        let mut by_name = HashMap::with_capacity(world.len());

        for player in world {
            let cell = GridCell::of(player.get_position(), cell_size);
            by_name.insert(player.get_name().to_string(), player.clone());
            cells.entry(cell).or_default().push(player);
        }

        Self {
            cells,
            by_name,
            on_voice,
            cell_size,
        }
    }

    pub fn empty(cell_size: f32) -> Self {
        Self {
            cells: HashMap::new(),
            by_name: HashMap::new(),
            on_voice: HashSet::new(),
            cell_size,
        }
    }

    /// The observer as the world last reported them, or `None` when they are authenticated
    /// but not in the game yet — a normal state rather than an error.
    pub fn observer(&self, gamertag: &str) -> Option<&PlayerEnum> {
        self.by_name.get(gamertag)
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

    pub fn is_on_voice(&self, gamertag: &str) -> bool {
        self.on_voice.contains(gamertag)
    }

    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }
}
