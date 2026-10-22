use common::structs::group_key::notice::{
    ProtectedGroupCheckingNotice, ProtectedGroupCodeNotice, ProtectedGroupEpochNotice,
    ProtectedGroupJoinStartedNotice, ProtectedGroupJoinerNotice, ProtectedGroupLostNotice,
    ProtectedGroupRequestNotice,
    ProtectedGroupRosterNotice, ProtectedGroupWaitingNotice,
};
use tauri::{AppHandle, Emitter};
use tauri_plugin_curia::curia;

use super::{CheckIdHex, ProtectedGroupEvent};
use crate::audio::{Cue, CueAnnouncer};

/// Tells the webview about a protected group. Carries the epoch, roster, codes, safety code and
/// lost reason only: keys never cross into the webview.
pub struct ProtectedGroupNotifier;

impl ProtectedGroupNotifier {
    pub const JOINED: &'static str = "protected-group:joined";
    pub const ROSTER: &'static str = "protected-group:roster";
    pub const EPOCH: &'static str = "protected-group:epoch";
    pub const LOST: &'static str = "protected-group:lost";
    pub const REQUEST: &'static str = "protected-group:request";
    pub const WAITING: &'static str = "protected-group:waiting";
    pub const CODE: &'static str = "protected-group:code";
    pub const CHECKING: &'static str = "protected-group:checking";
    pub const CHECK_FAILED: &'static str = "protected-group:check-failed";
    pub const REQUEST_ENDED: &'static str = "protected-group:request-ended";
    pub const ADMITTED: &'static str = "protected-group:admitted";
    pub const JOIN_STARTED: &'static str = "protected-group:join-started";

    /// A join request began from outside the window; the webview opens its dialog.
    pub fn join_started(app: &AppHandle, channel_id: &str) {
        if let Err(e) = app.emit(
            Self::JOIN_STARTED,
            ProtectedGroupJoinStartedNotice {
                channel_id: channel_id.to_string(),
            },
        ) {
            curia::warn!("could not tell the webview a join started", {
                channel_id: channel_id.to_string(),
                error: e.to_string(),
            });
        }
    }

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
            ProtectedGroupEvent::Request { joiner, name, cue } => {
                // On any page: the cards show only on the dashboard, and the sound is how a
                // member elsewhere knows to look.
                if *cue {
                    CueAnnouncer::new(app).play(Cue::JoinRequest);
                }
                app.emit(
                    Self::REQUEST,
                    ProtectedGroupRequestNotice {
                        joiner_socket: *joiner,
                        name: name.clone(),
                    },
                )
            }
            ProtectedGroupEvent::Waiting(count) => app.emit(
                Self::WAITING,
                ProtectedGroupWaitingNotice {
                    count: u32::try_from(*count).unwrap_or(u32::MAX),
                },
            ),
            ProtectedGroupEvent::Code {
                role,
                check_id,
                joiner,
                peer_name,
                code,
            } => app.emit(
                Self::CODE,
                ProtectedGroupCodeNotice {
                    role: role.as_str().to_string(),
                    check_id: CheckIdHex::encode(check_id),
                    joiner_socket: *joiner,
                    peer_name: peer_name.clone(),
                    code: code.clone(),
                },
            ),
            ProtectedGroupEvent::Checking {
                joiner,
                member_name,
            } => app.emit(
                Self::CHECKING,
                ProtectedGroupCheckingNotice {
                    joiner_socket: *joiner,
                    member_name: member_name.clone(),
                },
            ),
            ProtectedGroupEvent::CheckFailed { joiner } => {
                app.emit(Self::CHECK_FAILED, Self::joiner_notice(*joiner))
            }
            ProtectedGroupEvent::RequestEnded { joiner } => {
                app.emit(Self::REQUEST_ENDED, Self::joiner_notice(*joiner))
            }
            ProtectedGroupEvent::Admitted { joiner } => {
                app.emit(Self::ADMITTED, Self::joiner_notice(*joiner))
            }
        };
        if let Err(e) = sent {
            curia::warn!("could not notify the webview about a protected group", {
                error: e.to_string(),
            });
        }
    }

    fn joiner_notice(joiner: u64) -> ProtectedGroupJoinerNotice {
        ProtectedGroupJoinerNotice {
            joiner_socket: joiner,
        }
    }
}
