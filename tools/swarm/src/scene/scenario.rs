use clap::ValueEnum;

use crate::scene::{Placement, SceneConfig, SceneLayout};

/// The picture to compose.
///
/// Distances are expressed as fractions of the server's voice range so a scenario
/// keeps meaning what it says on a server that widened it: "in range" has to stay
/// inside the near tier, or the entry moves to the far list and the ring reads wrong.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Scenario {
    /// One player, close enough to read comfortably.
    OneInRange,
    /// Five players spread across the ring at mixed distances.
    FiveInRange,
    /// Twenty players — a crowded ring, all inside the near tier.
    TwentyInRange,
    /// A group alongside proximity: members connected into a channel, plus three
    /// players in range who are not in it.
    GroupAndRange,
}

impl Scenario {
    // Any closer than this and a label sits under the centre pill.
    const NEAREST: f32 = 0.12;

    // Short of the near-tier boundary. Landing on it puts an entry in the far list on
    // a rounding difference, which changes how it renders for no visible reason.
    const FURTHEST_NEAR: f32 = 0.94;

    pub fn resolve(&self, config: &SceneConfig) -> Result<SceneLayout, anyhow::Error> {
        let range = config.voice_range;
        match self {
            Self::OneInRange => {
                let names = config.take_names(0, 1)?;
                Ok(SceneLayout::staged_only(vec![Placement::new(
                    names[0].clone(),
                    35.0,
                    range * 0.18,
                    0.0,
                )]))
            }
            Self::FiveInRange => {
                let names = config.take_names(0, 5)?;
                Ok(SceneLayout::staged_only(Placement::ring(
                    &names,
                    range * Self::NEAREST,
                    range * 0.78,
                )))
            }
            Self::TwentyInRange => {
                let names = config.take_names(0, 20)?;
                Ok(SceneLayout::staged_only(Placement::ring(
                    &names,
                    range * Self::NEAREST,
                    range * Self::FURTHEST_NEAR,
                )))
            }
            Self::GroupAndRange => {
                let in_range = config.take_names(0, 3)?;
                let members = config.take_names(3, config.group_bots)?;
                Ok(SceneLayout::with_group(
                    Placement::ring(&in_range, range * 0.15, range * 0.6),
                    members,
                    config.group_name.clone(),
                ))
            }
        }
    }
}
