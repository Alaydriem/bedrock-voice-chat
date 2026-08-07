/**
 * A group's name at birth.
 *
 * Every group the client made was called "New group", so a user who declined to rename ended
 * up with a list of rows that were indistinguishable from each other. A name drawn from two
 * word lists is a name the moment it exists, and is still renameable by anyone who cares.
 *
 * 64 adjectives against 64 nouns is 4,096 names. That is small enough that a busy server will
 * eventually collide, which is why `next` is given the names already in use.
 */

/** A source of indices below a bound. Injected only so the draw can be pinned under test. */
export type RandomInt = (bound: number) => number;

export class GroupName {
  static readonly ADJECTIVES: readonly string[] = [
    "Obsidian", "Netherite", "Redstone", "Amethyst", "Emerald", "Diamond", "Copper", "Golden",
    "Iron", "Lapis", "Quartz", "Prismarine", "Deepslate", "Sculk", "Blackstone", "Basalt",
    "Calcite", "Tuff", "Andesite", "Granite", "Diorite", "Mossy", "Cracked", "Chiseled",
    "Polished", "Weathered", "Oxidized", "Waxed", "Glowing", "Blazing", "Frosted", "Frozen",
    "Molten", "Ember", "Twilight", "Midnight", "Sunlit", "Gilded", "Verdant", "Crimson",
    "Warped", "Azure", "Violet", "Amber", "Ivory", "Onyx", "Cobalt", "Scarlet",
    "Teal", "Lush", "Dripstone", "Mangrove", "Cherry", "Bamboo", "Spruce", "Birch",
    "Acacia", "Cobbled", "Rooted", "Ancient", "Echoing", "Silent", "Hollow", "Enchanted",
  ];

  static readonly NOUNS: readonly string[] = [
    "Ocelots", "Ravagers", "Foxes", "Wardens", "Striders", "Bees", "Axolotls", "Allays",
    "Piglins", "Creepers", "Endermen", "Guardians", "Llamas", "Parrots", "Pandas", "Dolphins",
    "Turtles", "Wolves", "Goats", "Ghasts", "Blazes", "Shulkers", "Vindicators", "Pillagers",
    "Drowned", "Husks", "Sniffers", "Camels", "Frogs", "Tadpoles", "Bats", "Cats",
    "Rabbits", "Squids", "Hoglins", "Zoglins", "Silverfish", "Phantoms", "Vexes", "Evokers",
    "Illagers", "Witches", "Skeletons", "Spiders", "Slimes", "Breezes", "Wanderers", "Cartographers",
    "Miners", "Raiders", "Explorers", "Spelunkers", "Nomads", "Outriders", "Scouts", "Sentries",
    "Traders", "Crafters", "Smelters", "Anglers", "Beekeepers", "Herders", "Voyagers", "Prospectors",
  ];

  /** Fresh draws attempted before the name is kept and numbered instead. */
  private static readonly TRIES = 12;

  /**
   * A name for a new group, avoiding the ones already in use.
   *
   * @param taken Names of the groups on this server. Compared trimmed and case-insensitively.
   * @param rand Index source. Production callers leave this alone.
   */
  static next(taken: readonly string[] = [], rand: RandomInt = GroupName.randomInt): string {
    const used = new Set(taken.map((name) => name.trim().toLowerCase()));
    let name = "";
    for (let attempt = 0; attempt < GroupName.TRIES; attempt++) {
      name = GroupName.draw(rand);
      if (!used.has(name.toLowerCase())) return name;
    }
    // Every draw collided. `used` is finite, so counting up from 2 always terminates.
    for (let ordinal = 2; ; ordinal++) {
      const numbered = `${name} ${ordinal}`;
      if (!used.has(numbered.toLowerCase())) return numbered;
    }
  }

  /**
   * An index below `bound`, without modulo bias.
   *
   * Taking `value % bound` of a raw 32-bit draw favours the low indices whenever `bound` does
   * not divide 2^32. Values in the ragged tail above the last whole multiple are discarded and
   * redrawn instead, which costs nothing for the list sizes here and stays correct if they change.
   */
  static randomInt(bound: number): number {
    if (!Number.isInteger(bound) || bound < 1) {
      throw new RangeError(`bound must be a positive integer, got ${bound}`);
    }
    const ceiling = Math.floor(0x1_0000_0000 / bound) * bound;
    const buffer = new Uint32Array(1);
    for (;;) {
      crypto.getRandomValues(buffer);
      if (buffer[0] < ceiling) return buffer[0] % bound;
    }
  }

  private static draw(rand: RandomInt): string {
    const adjective = GroupName.ADJECTIVES[rand(GroupName.ADJECTIVES.length)];
    const noun = GroupName.NOUNS[rand(GroupName.NOUNS.length)];
    return `${adjective} ${noun}`;
  }
}
