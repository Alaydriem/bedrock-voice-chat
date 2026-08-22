pub mod block_coordinate;
pub mod coordinate;
pub mod data;
pub mod identity;
pub mod orientation;
pub mod player;

pub use block_coordinate::BlockCoordinate;
pub use coordinate::Coordinate;
pub use data::GameData;
pub use identity::UploaderIdentity;
pub use orientation::Orientation;
pub use player::Player;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash, ts_rs::TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
#[cfg_attr(
    feature = "server",
    derive(sea_orm::EnumIter, sea_orm::DeriveActiveEnum, clap::ValueEnum)
)]
#[cfg_attr(feature = "server", sea_orm(rs_type = "String", db_type = "Text"))]
pub enum Game {
    #[serde(rename = "minecraft")]
    #[cfg_attr(feature = "server", sea_orm(string_value = "minecraft"))]
    Minecraft,
}

impl Game {
    pub fn as_str(&self) -> &'static str {
        match self {
            Game::Minecraft => "minecraft",
        }
    }

    // Inverse of `as_str`: the wire/CN tag form back into the enum.
    pub fn from_tag(tag: &str) -> Option<Game> {
        match tag {
            "minecraft" => Some(Game::Minecraft),
            _ => None,
        }
    }

    // The channel-membership / cert-CN key for a player: `game:gamertag`
    // (e.g. "minecraft:Alice"). This is the single source of truth for the key
    // form that ChannelCollection, player_channel, and the control routes share.
    pub fn membership_key(&self, gamertag: &str) -> crate::PlayerIdentity {
        crate::PlayerIdentity::new(self.clone(), gamertag)
    }

    /// The bare gamertag out of a canonical identity, **for display only**.
    ///
    /// Nothing may be keyed on this. Two players in different games can share a gamertag,
    /// so stripping the prefix merges two distinct identities into one — which is the exact
    /// collision `membership_key` exists to prevent. Use it where a human reads the result
    /// and nothing looks it up: a log line, a diagnostics table, a label.
    ///
    /// A string with no known game prefix is returned unchanged, so a value that was never
    /// canonical in the first place is not silently truncated at some other colon.
    pub fn display_name(identity: &str) -> &str {
        match identity.split_once(':') {
            Some((tag, name)) if Self::from_tag(tag).is_some() => name,
            _ => identity,
        }
    }
}

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "server")]
impl<'r> rocket::request::FromParam<'r> for Game {
    type Error = &'r str;

    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        Game::from_tag(param).ok_or(param)
    }
}

// Lets a route take `game` as a query parameter, not just a path segment. Both
// positions parse through `from_tag`, so an unknown tag is rejected the same way
// wherever it arrives.
#[cfg(feature = "server")]
impl<'v> rocket::form::FromFormField<'v> for Game {
    fn from_value(field: rocket::form::ValueField<'v>) -> rocket::form::Result<'v, Self> {
        Game::from_tag(field.value)
            .ok_or_else(|| rocket::form::Error::validation("not a known game").into())
    }
}
