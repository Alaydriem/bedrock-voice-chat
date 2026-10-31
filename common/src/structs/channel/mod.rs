pub mod code;
pub mod collection;
pub mod event;
pub mod events;
pub mod naming;

pub use code::GroupCode;
pub use collection::ChannelCollection;
pub use event::ChannelEvent;
pub use events::ChannelEvents;
pub use naming::GroupName;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct Channel {
    id: String,
    pub name: String,
    // Rendered as the canonical string for TypeScript, so the webview keeps comparing
    // `creator` and `players` as strings and no new binding file appears.
    #[ts(as = "Vec<String>")]
    pub players: Vec<crate::PlayerIdentity>,
    #[ts(as = "String")]
    pub creator: crate::PlayerIdentity,
    /// End-to-end encrypted. The server only routes and lists it; it never holds the password.
    /// A server that predates this field sends nothing, which reads as unprotected.
    #[serde(default)]
    pub protected: bool,
    /// Set only by the Simple Voice Chat bridge, for a channel linked to a NORMAL or ISOLATED
    /// SVC group. Speech in it reaches channel members only, never nearby players.
    #[serde(default)]
    members_only: bool,
}

impl Channel {
    pub fn new(name: String, creator: crate::PlayerIdentity) -> Self {
        Self::with_id(GroupCode::generate(), name, creator)
    }

    pub fn new_protected(name: String, creator: crate::PlayerIdentity) -> Self {
        Self::new(name, creator).into_protected()
    }

    pub fn into_protected(self) -> Self {
        Self {
            protected: true,
            ..self
        }
    }

    pub fn is_protected(&self) -> bool {
        self.protected
    }

    pub fn into_members_only(self) -> Self {
        Self {
            members_only: true,
            ..self
        }
    }

    pub fn is_members_only(&self) -> bool {
        self.members_only
    }

    /// A channel carrying a caller-supplied code, for the create path that has to
    /// check the code is free before it commits to one.
    pub fn with_id(id: String, name: String, creator: crate::PlayerIdentity) -> Self {
        Self {
            id,
            name,
            players: Vec::new(),
            creator,
            protected: false,
            members_only: false,
        }
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn contains(&self, identity: &crate::PlayerIdentity) -> bool {
        self.players.iter().any(|p| p == identity)
    }

    pub fn add_player(&mut self, identity: crate::PlayerIdentity) {
        if !self.players.contains(&identity) {
            self.players.push(identity);
        }
    }

    pub fn remove_player(&mut self, identity: &crate::PlayerIdentity) {
        self.players.retain(|p| p != identity);
    }

    pub fn rename(&mut self, name: String) {
        self.name = name;
    }
}
