use std::collections::{BTreeSet, HashSet};
use std::time::Duration;

// What the watch remembers between ticks.
//
// Split from the ticker so the decisions — has the set changed, is this world
// worth warning about yet — are testable without a clock or a player cache.
// Elapsed time is a parameter rather than something read here for the same
// reason.
pub struct WorldWatchState {
    current: Option<BTreeSet<String>>,
    ever: HashSet<String>,
    warned: HashSet<String>,
}

impl WorldWatchState {
    // Long enough that a server restarting into an empty world does not warn
    // about a configuration that is correct, short enough that an operator who
    // mistyped a world id learns it during the session they made the mistake in.
    pub const GRACE: Duration = Duration::from_secs(300);

    pub fn new() -> Self {
        Self {
            current: None,
            ever: HashSet::new(),
            warned: HashSet::new(),
        }
    }

    // The live set, when it differs from the previous observation.
    //
    // `current` starts as `None` rather than an empty set so the first
    // observation always reports: a server that starts with players already in a
    // world would otherwise never log what it hosts.
    pub fn observe(&mut self, worlds: &[String]) -> Option<Vec<String>> {
        let observed: BTreeSet<String> = worlds.iter().cloned().collect();

        for world in &observed {
            self.ever.insert(world.clone());
        }

        if self.current.as_ref() == Some(&observed) {
            return None;
        }

        let reported = observed.iter().cloned().collect();
        self.current = Some(observed);
        Some(reported)
    }

    // Configured worlds no player has ever been seen in, each reported once.
    //
    // Membership is tested against every world ever observed rather than the
    // current set: a world that emptied out was still real, and warning on it
    // would tell an operator their config is wrong when it is right.
    pub fn unwarned_missing(
        &mut self,
        configured: &[(String, String)],
        elapsed: Duration,
    ) -> Vec<(String, String)> {
        if elapsed < Self::GRACE {
            return Vec::new();
        }

        let mut out = Vec::new();
        for (label, world) in configured {
            if self.ever.contains(world) || self.warned.contains(world) {
                continue;
            }
            self.warned.insert(world.clone());
            out.push((label.clone(), world.clone()));
        }

        out
    }
}

impl Default for WorldWatchState {
    fn default() -> Self {
        Self::new()
    }
}
