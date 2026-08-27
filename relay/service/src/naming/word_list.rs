// The vocabulary assigned names are drawn from.
//
// Minecraft's own nouns — blocks, mobs, biomes — because the name is the operator's
// public address for a Minecraft voice server and should read like one.
//
// Common nouns only. Mojang and Microsoft marks are excluded deliberately: the name
// sits in a zone this project owns, and a product name there is a trademark problem
// nobody wants while Java parity work is still pending approval.
pub struct WordList;

impl WordList {
    pub const ADJECTIVES: &'static [&'static str] = &[
        "amber",
        "ancient",
        "azure",
        "blazing",
        "bright",
        "calm",
        "crimson",
        "dusty",
        "eager",
        "emerald",
        "frosted",
        "gentle",
        "gilded",
        "glacial",
        "golden",
        "hidden",
        "humming",
        "lucky",
        "mellow",
        "misty",
        "quiet",
        "rustic",
        "shaded",
        "silent",
        "sunlit",
        "swift",
        "tidy",
        "velvet",
        "wandering",
        "wild",
    ];

    pub const NOUNS: &'static [&'static str] = &[
        "allay",
        "anvil",
        "axolotl",
        "beacon",
        "cauldron",
        "creeper",
        "dolphin",
        "ender",
        "ghast",
        "glowstone",
        "hopper",
        "lantern",
        "lectern",
        "obsidian",
        "ocelot",
        "parrot",
        "piglin",
        "prismarine",
        "pufferfish",
        "quartz",
        "redstone",
        "sculk",
        "shulker",
        "slime",
        "strider",
        "turtle",
        "vindicator",
        "warden",
        "wolf",
        "zombie",
    ];

    pub const PLACES: &'static [&'static str] = &[
        "badlands",
        "basalt",
        "beach",
        "birchwood",
        "bluffs",
        "caverns",
        "cliffside",
        "deepslate",
        "delta",
        "dripstone",
        "grove",
        "highlands",
        "hollow",
        "jungle",
        "lagoon",
        "meadow",
        "mesa",
        "moor",
        "oasis",
        "outpost",
        "ravine",
        "reef",
        "savanna",
        "shoals",
        "spires",
        "taiga",
        "thicket",
        "tundra",
        "valley",
        "wetlands",
    ];

    // Substrings that disqualify a word. Checked against the assembled list by test
    // rather than at runtime: the list is a constant, so a violation is a build-time
    // fact, not a request-time one.
    const DENY: &'static [&'static str] = &["minecraft", "mojang", "microsoft", "xbox", "bedrock"];

    pub fn is_clean(word: &str) -> bool {
        let lowered = word.to_ascii_lowercase();
        !Self::DENY.iter().any(|bad| lowered.contains(bad))
    }
}
