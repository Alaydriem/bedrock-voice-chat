use common::structs::group_key::notice::{
    ProtectedGroupEpochNotice, ProtectedGroupLostNotice, ProtectedGroupRosterNotice,
};
use tauri::{AppHandle, Emitter};

use super::ProtectedGroupEvent;

/// Tells the webview about a protected group. Carries the epoch, roster, safety code and lost
/// reason only: keys and the password never cross into the webview.
pub struct ProtectedGroupNotifier;

impl ProtectedGroupNotifier {
    pub const JOINED: &'static str = "protected-group:joined";
    pub const ROSTER: &'static str = "protected-group:roster";
    pub const EPOCH: &'static str = "protected-group:epoch";
    pub const LOST: &'static str = "protected-group:lost";

    pub fn notify(app: &AppHandle, event: &ProtectedGroupEvent) {
        let sent = match event {
            ProtectedGroupEvent::Joined { epoch, safety_code } => app.emit(
                Self::JOINED,
                ProtectedGroupEpochNotice {
                    epoch: *epoch,
                    safety_code: safety_code.clone(),
                },
            ),
            ProtectedGroupEvent::RosterChanged(members) => app.emit(
                Self::ROSTER,
                ProtectedGroupRosterNotice {
                    members: members.clone(),
                },
            ),
            ProtectedGroupEvent::EpochChanged { epoch, safety_code } => app.emit(
                Self::EPOCH,
                ProtectedGroupEpochNotice {
                    epoch: *epoch,
                    safety_code: safety_code.clone(),
                },
            ),
            ProtectedGroupEvent::Lost(reason) => app.emit(
                Self::LOST,
                ProtectedGroupLostNotice {
                    reason: reason.as_str().to_string(),
                },
            ),
        };
        if let Err(e) = sent {
            log::warn!("could not notify the webview about a protected group: {e}");
        }
    }
}
