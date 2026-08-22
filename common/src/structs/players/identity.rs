use serde::de::{Deserializer, Error as DeError};
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};

use crate::Game;

/// The canonical identity a player is keyed on everywhere: a game and a gamertag.
///
/// The fields are private and the only ways in are `new` and `FromStr`, so a bare
/// gamertag cannot become one. Channel membership, the certificate Common Name, the
/// position caches and audio routing all key on this value, and a bare gamertag reaching
/// any of them matches nothing rather than failing.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct PlayerIdentity {
    game: Game,
    gamertag: String,
}

/// Why a string is not a canonical identity.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PlayerIdentityError {
    #[error("no game prefix in {0:?}")]
    NoPrefix(String),
    #[error("unknown game prefix {0:?}")]
    UnknownGame(String),
    #[error("empty gamertag")]
    EmptyGamertag,
}

impl PlayerIdentity {
    pub fn new(game: Game, gamertag: impl Into<String>) -> Self {
        Self {
            game,
            gamertag: gamertag.into(),
        }
    }

    pub fn game(&self) -> &Game {
        &self.game
    }

    /// The bare gamertag, for display and for the database, gamerpic and alias tables
    /// that key on it. Nothing that keys on an identity may use this.
    pub fn gamertag(&self) -> &str {
        &self.gamertag
    }
}

impl std::fmt::Display for PlayerIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.game.as_str(), self.gamertag)
    }
}

impl std::str::FromStr for PlayerIdentity {
    type Err = PlayerIdentityError;

    /// Splits on the FIRST colon only. A gamertag may contain a colon; the game prefix
    /// may not.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (tag, gamertag) = value
            .split_once(':')
            .ok_or_else(|| PlayerIdentityError::NoPrefix(value.to_string()))?;

        if gamertag.is_empty() {
            return Err(PlayerIdentityError::EmptyGamertag);
        }

        let game =
            Game::from_tag(tag).ok_or_else(|| PlayerIdentityError::UnknownGame(tag.to_string()))?;

        Ok(Self {
            game,
            gamertag: gamertag.to_string(),
        })
    }
}

#[derive(Deserialize)]
struct BinaryWire {
    game: Game,
    gamertag: String,
}

impl Serialize for PlayerIdentity {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // JSON, RON and the ts-rs surface keep the canonical string, so the webview and
        // every stored channel see the same shape they always did.
        if serializer.is_human_readable() {
            return serializer.serialize_str(&self.to_string());
        }

        // postcard carries the game as a discriminant instead of as its prefix text.
        let mut state = serializer.serialize_struct("PlayerIdentity", 2)?;
        state.serialize_field("game", &self.game)?;
        state.serialize_field("gamertag", &self.gamertag)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for PlayerIdentity {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            let value = String::deserialize(deserializer)?;
            return value.parse().map_err(D::Error::custom);
        }

        let wire = BinaryWire::deserialize(deserializer)?;
        Ok(Self {
            game: wire.game,
            gamertag: wire.gamertag,
        })
    }
}

// Described as a string, matching the human-readable serde form, so the generated OpenAPI
// document agrees with what an HTTP client actually sends and receives.
#[cfg(feature = "openapi")]
impl schemars::JsonSchema for PlayerIdentity {
    fn schema_name() -> String {
        "PlayerIdentity".to_string()
    }

    fn json_schema(generator: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        <String as schemars::JsonSchema>::json_schema(generator)
    }
}
