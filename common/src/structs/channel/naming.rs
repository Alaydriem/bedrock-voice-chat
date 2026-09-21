use rand::RngExt;

/// A group's name at birth.
///
/// 64 adjectives against 64 nouns is 4,096 names, which a busy server eventually
/// exhausts, so `next` is given the names already in use and numbers the last draw
/// when every attempt collides.
pub struct GroupName;

impl GroupName {
    pub const ADJECTIVES: &'static [&'static str] = &[
        "Obsidian",
        "Netherite",
        "Redstone",
        "Amethyst",
        "Emerald",
        "Diamond",
        "Copper",
        "Golden",
        "Iron",
        "Lapis",
        "Quartz",
        "Prismarine",
        "Deepslate",
        "Sculk",
        "Blackstone",
        "Basalt",
        "Calcite",
        "Tuff",
        "Andesite",
        "Granite",
        "Diorite",
        "Mossy",
        "Cracked",
        "Chiseled",
        "Polished",
        "Weathered",
        "Oxidized",
        "Waxed",
        "Glowing",
        "Blazing",
        "Frosted",
        "Frozen",
        "Molten",
        "Ember",
        "Twilight",
        "Midnight",
        "Sunlit",
        "Gilded",
        "Verdant",
        "Crimson",
        "Warped",
        "Azure",
        "Violet",
        "Amber",
        "Ivory",
        "Onyx",
        "Cobalt",
        "Scarlet",
        "Teal",
        "Lush",
        "Dripstone",
        "Mangrove",
        "Cherry",
        "Bamboo",
        "Spruce",
        "Birch",
        "Acacia",
        "Cobbled",
        "Rooted",
        "Ancient",
        "Echoing",
        "Silent",
        "Hollow",
        "Enchanted",
    ];

    pub const NOUNS: &'static [&'static str] = &[
        "Ocelots",
        "Ravagers",
        "Foxes",
        "Wardens",
        "Striders",
        "Bees",
        "Axolotls",
        "Allays",
        "Piglins",
        "Creepers",
        "Endermen",
        "Guardians",
        "Llamas",
        "Parrots",
        "Pandas",
        "Dolphins",
        "Turtles",
        "Wolves",
        "Goats",
        "Ghasts",
        "Blazes",
        "Shulkers",
        "Vindicators",
        "Pillagers",
        "Drowned",
        "Husks",
        "Sniffers",
        "Camels",
        "Frogs",
        "Tadpoles",
        "Bats",
        "Cats",
        "Rabbits",
        "Squids",
        "Hoglins",
        "Zoglins",
        "Silverfish",
        "Phantoms",
        "Vexes",
        "Evokers",
        "Illagers",
        "Witches",
        "Skeletons",
        "Spiders",
        "Slimes",
        "Breezes",
        "Wanderers",
        "Cartographers",
        "Miners",
        "Raiders",
        "Explorers",
        "Spelunkers",
        "Nomads",
        "Outriders",
        "Scouts",
        "Sentries",
        "Traders",
        "Crafters",
        "Smelters",
        "Anglers",
        "Beekeepers",
        "Herders",
        "Voyagers",
        "Prospectors",
    ];

    // Fresh draws attempted before the last one is kept and numbered instead.
    const TRIES: usize = 12;

    /// A name no entry of `taken` already uses.
    pub fn next(taken: &[String]) -> String {
        let mut rng = rand::rng();
        let mut draw = |bound: usize| rng.random_range(0..bound);
        Self::next_with(taken, &mut draw)
    }

    /// The same draw against a caller-supplied index source, so a test can pin it.
    pub fn next_with(taken: &[String], rand: &mut impl FnMut(usize) -> usize) -> String {
        let used: std::collections::HashSet<String> = taken
            .iter()
            .map(|name| name.trim().to_lowercase())
            .collect();

        let mut name = String::new();
        for _ in 0..Self::TRIES {
            name = Self::draw(rand);
            if !used.contains(&name.to_lowercase()) {
                return name;
            }
        }

        // Every draw collided. `used` is finite, so counting up always terminates.
        for ordinal in 2.. {
            let numbered = format!("{name} {ordinal}");
            if !used.contains(&numbered.to_lowercase()) {
                return numbered;
            }
        }
        unreachable!()
    }

    fn draw(rand: &mut impl FnMut(usize) -> usize) -> String {
        let adjective = Self::ADJECTIVES[rand(Self::ADJECTIVES.len())];
        let noun = Self::NOUNS[rand(Self::NOUNS.len())];
        format!("{adjective} {noun}")
    }
}
