use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::mode::ChatMode;

/// Whether chat can carry a message right now, and why not when it cannot.
///
/// Deliberately a single answer covering every source rather than one flag per transport. A
/// proxy session and a mod reporting positions are two ways of reaching the same world, and
/// the composer only ever needs to know whether typing will accomplish anything.
///
/// Polled rather than pushed: liveness has several independent inputs and a missed event on
/// any of them would leave the composer permanently wrong in one direction or the other.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ChatAvailability {
    pub available: bool,
    /// Which implementation is answering. `None` when nothing is.
    pub mode: Option<ChatMode>,
    /// Shown in the composer when unavailable. Absent when chat works.
    pub reason: Option<String>,
    /// The world's display name where one is known.
    pub world_name: Option<String>,
}

impl ChatAvailability {
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            available: false,
            mode: None,
            reason: Some(reason.into()),
            world_name: None,
        }
    }

    pub fn local(world_name: Option<String>) -> Self {
        Self {
            available: true,
            mode: Some(ChatMode::Local),
            reason: None,
            world_name,
        }
    }
}
