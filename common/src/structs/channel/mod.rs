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
}

impl Channel {
    pub fn new(name: String, creator: crate::PlayerIdentity) -> Self {
        Self::with_id(GroupCode::generate(), name, creator)
    }

    /// A channel carrying a caller-supplied code, for the create path that has to
    /// check the code is free before it commits to one.
    pub fn with_id(id: String, name: String, creator: crate::PlayerIdentity) -> Self {
        Self {
            id,
            name,
            players: Vec::new(),
            creator,
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
