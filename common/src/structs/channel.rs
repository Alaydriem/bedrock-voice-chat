use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::channel_player::ChannelPlayer;
use crate::Game;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct Channel {
    id: String,
    pub name: String,
    pub players: Vec<ChannelPlayer>,
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

    pub fn add_player(&mut self, player: ChannelPlayer) -> Result<(), anyhow::Error> {
        if !self.players.iter().any(|p| p.name == player.name) {
            self.players.push(player);
        }

        Ok(())
    }

    pub fn remove_player(&mut self, name: &str) -> Result<(), anyhow::Error> {
        self.players.retain(|p| p.name != name);
        Ok(())
    }

    pub fn rename(&mut self, name: String) {
        self.name = name;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum ChannelEvents {
    Join,
    Leave,
    Create,
    Delete,
    Rename,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ChannelEvent {
    pub event: ChannelEvents,
    #[serde(default)]
    pub game: Option<Game>,
}

impl ChannelEvent {
    pub fn new(event: ChannelEvents) -> Self {
        Self { event, game: None }
    }
}
