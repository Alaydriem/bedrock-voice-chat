pub mod collection;
pub mod event;
pub mod events;

pub use collection::ChannelCollection;
pub use event::ChannelEvent;
pub use events::ChannelEvents;

use nanoid::nanoid;
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
        Self {
            id: nanoid!(),
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
