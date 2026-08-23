/// Names for the channels a scene creates.
///
/// A speaker needs a channel of its own so its audio stays spatial, and that channel's
/// name is not addressed to the observer — they are not a member and will never see it
/// in their rail. It still reaches the server's channel list, its logs, and any admin
/// view of either, so it should read like somewhere in a world rather than like the
/// tool that made it.
pub struct ChannelNames;

impl ChannelNames {
    // One per speaker, assigned by position so a given scene names its channels the same
    // way on every run.
    const SOLO: [&'static str; 12] = [
        "Torchlit Shaft",
        "Deepslate Seam",
        "Mossy Hollow",
        "Lantern Row",
        "Slime Chunk",
        "Amethyst Geode",
        "Dripstone Cave",
        "Lush Ravine",
        "Spruce Ridge",
        "Copper Vein",
        "Basalt Delta",
        "Warped Grove",
    ];

    // Offered to an operator choosing `group_name`, and the first is the default.
    pub const GROUPS: [&'static str; 8] = [
        "End Raid",
        "Nether Run",
        "Mining Crew",
        "Redstone Guild",
        "Elytra Wing",
        "Stronghold Dig",
        "Ancient Debris",
        "Wither Squad",
    ];

    /// A distinct channel for the nth speaker.
    ///
    /// Numbered past the end of the list rather than wrapping onto a name already in
    /// use: two speakers sharing a channel hear each other without attenuation, which is
    /// the one thing a proximity scene must not quietly introduce.
    pub fn solo(index: usize) -> String {
        let name = Self::SOLO[index % Self::SOLO.len()];
        let lap = index / Self::SOLO.len();
        if lap == 0 {
            return name.to_string();
        }
        format!("{} {}", name, lap + 1)
    }

    pub fn default_group() -> String {
        Self::GROUPS[0].to_string()
    }
}
