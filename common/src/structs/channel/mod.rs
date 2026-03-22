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
    pub players: Vec<String>,
    pub creator: String,
}

impl Channel {
    pub fn new(name: String, creator: String) -> Self {
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

    pub fn contains(&self, name: &str) -> bool {
        self.players.iter().any(|p| p == name)
    }

    pub fn add_player(&mut self, name: String) -> Result<(), anyhow::Error> {
        if !self.players.contains(&name) {
            self.players.push(name);
        }

        Ok(())
    }

    pub fn remove_player(&mut self, name: String) -> Result<(), anyhow::Error> {
        if let Some(id) = self.players.iter().position(|each| *each == name) {
            self.players.remove(id);
        }

        Ok(())
    }

    pub fn rename(&mut self, name: String) {
        self.name = name;
    }
}
