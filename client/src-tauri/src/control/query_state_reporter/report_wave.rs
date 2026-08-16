use std::collections::BTreeSet;

use super::ControlStateSignal;

// One debounced wave of coalesced signals.
#[derive(Default)]
pub(super) struct ReportWave {
    pub(super) self_state: bool,
    pub(super) preferences: bool,
    pub(super) sync: bool,
    pub(super) sync_targets: BTreeSet<String>,
}

impl ReportWave {
    pub(super) fn fold(&mut self, signal: ControlStateSignal) {
        match signal {
            ControlStateSignal::SelfState => self.self_state = true,
            ControlStateSignal::Preferences => self.preferences = true,
            ControlStateSignal::Sync { targets } => {
                self.sync = true;
                self.sync_targets.extend(targets);
            }
        }
    }

    pub(super) fn fold_all(&mut self) {
        self.self_state = true;
        self.preferences = true;
    }
}
