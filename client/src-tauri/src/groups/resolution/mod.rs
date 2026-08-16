use common::structs::channel::Channel;

mod error;

pub use error::GroupError;

/// Which group a name or a player refers to.
pub struct GroupResolution;

impl GroupResolution {
    pub fn by_name<'a>(channels: &'a [Channel], name: &str) -> Result<&'a Channel, GroupError> {
        let exact: Vec<&Channel> = channels.iter().filter(|c| c.name == name).collect();
        // An exact match wins outright, so a deliberate distinction between two groups whose names
        // differ only in case is honoured rather than resolved by list order.
        let matches: Vec<&Channel> = if exact.is_empty() {
            channels
                .iter()
                .filter(|c| c.name.eq_ignore_ascii_case(name))
                .collect()
        } else {
            exact
        };

        match matches.len() {
            0 => Err(GroupError::NotFound(name.to_string())),
            1 => Ok(matches[0]),
            count => Err(GroupError::Ambiguous {
                name: name.to_string(),
                count,
            }),
        }
    }

    /// The group a player is in, if any.
    ///
    /// `player` must be the canonical `game:gamertag` form, which is what membership is keyed on.
    /// A bare gamertag matches nothing, which would turn a leave into a no-op reporting success.
    pub fn containing<'a>(channels: &'a [Channel], player: &str) -> Option<&'a Channel> {
        channels.iter().find(|channel| channel.contains(player))
    }
}
