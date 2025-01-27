use nanoid::nanoid;
use serde::{ Serialize, Deserialize };
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct Channel {
    id: String,
    pub name: String,
    pub players: Vec<String>,
    pub creator: String,
}

impl Channel {
    /// Creates a new channel with a randomly generated ID
    pub fn new(name: String, creator: String) -> Self {
        Self {
            id: nanoid!(),
            name,
            players: Vec::new(),
            creator,
        }
    }

    /// Returns the channel ID
    pub fn id(&self) -> String {
        self.id.clone()
    }

    /// Adds a player to the channel
    pub fn add_player(&mut self, name: String) -> Result<(), anyhow::Error> {
        if !self.players.contains(&name) {
            self.players.push(name);
        }

        Ok(())
    }

    /// Removes a player from the channel
    pub fn remove_player(&mut self, name: String) -> Result<(), anyhow::Error> {
        if self.players.contains(&name) {
            if let Some(id) = self.players.iter().position(|each| *each == name) {
                self.players.remove(id);
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum ChannelEvents {
    Join,
    Leave,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ChannelEvent {
    pub event: ChannelEvents,
}

impl ChannelEvent {
    pub fn new(event: ChannelEvents) -> Self {
        Self { event }
    }
}
