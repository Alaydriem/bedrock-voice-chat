use crate::scene::{ChatBeat, Deck};

/// Produces a chat log that reads like a server with people on it.
///
/// Varied but not random: the sequence is a function of the seed and the position in it, so the
/// same scene produces the same conversation every run. A screenshot retaken after a layout
/// change should differ in the layout and nothing else, and a genuinely random log would make
/// every retake a fresh comparison problem.
///
/// Every list is dealt from a shuffled deck rather than sampled independently — see [`Deck`]
/// for why the obvious approach reads badly.
pub struct ChatScript {
    names: Vec<String>,
    state: u64,
    beat: u64,
    speakers: Deck,
    targets: Deck,
    subjects: Deck,
    lines: Deck,
    directed: Deck,
    deaths: Deck,
    advancements: Deck,
    comings: Deck,
    visitors: Deck,
}

impl ChatScript {
    // Lines somebody types without addressing anyone.
    const LINES: [&'static str; 20] = [
        "anyone got spare torches",
        "im at the mineshaft under spawn",
        "who took the diamonds out of the shulker",
        "brb feeding the dogs",
        "creeper got my chest again",
        "portal's lit, meet me at the nether hub",
        "found a geode two chunks east",
        "this ravine goes on forever",
        "need three more iron for a bucket",
        "skeleton with a bow up on the ledge, careful",
        "the villagers escaped again",
        "trading hall is done, come look",
        "i keep falling in the same hole",
        "elytra run in five?",
        "my pickaxe just broke, classic",
        "lava pool right where you're digging",
        "afk a sec",
        "pretty sure the warden heard that",
        "shulker box is in the ender chest",
        "who left a jukebox in the mine",
    ];

    // Lines addressed to somebody else, which is what makes a log read as a conversation
    // rather than as a list.
    const DIRECTED: [&'static str; 10] = [
        "{to} you dropped your netherite",
        "{to} behind you",
        "{to} bring pearls",
        "{to} did you light the portal",
        "{to} i left you a stack of arrows",
        "{to} that was you screaming right",
        "{to} meet me at the geode",
        "{to} your dog followed me home",
        "{to} nice shot",
        "{to} the chest by the furnace is yours",
    ];

    const DEATHS: [&'static str; 12] = [
        "{who} was blown up by a Creeper",
        "{who} was slain by a Zombie",
        "{who} was shot by a Skeleton",
        "{who} fell from a high place",
        "{who} tried to swim in lava",
        "{who} drowned",
        "{who} went up in flames",
        "{who} was squashed by a falling anvil",
        "{who} was impaled on a stalagmite",
        "{who} was pricked to death",
        "{who} discovered the floor was lava",
        "{who} starved to death",
    ];

    const ADVANCEMENTS: [&'static str; 6] = [
        "{who} has made the advancement [Diamonds!]",
        "{who} has made the advancement [Hot Stuff]",
        "{who} has made the advancement [We Need to Go Deeper]",
        "{who} has made the advancement [Monster Hunter]",
        "{who} has made the advancement [Sky's the Limit]",
        "{who} has made the advancement [The Cutest Predator]",
    ];

    const COMINGS: [&'static str; 2] = ["{who} joined the game", "{who} left the game"];

    // Join and leave lines name these rather than anybody in the scene. A player who "left the
    // game" and then carries on chatting is the one system line a reader can catch as false,
    // and the staged roster is by definition still standing there.
    const VISITORS: [&'static str; 6] = [
        "GravelGus",
        "HuskHarriet",
        "SaplingSid",
        "BoneMealBo",
        "CartographerCal",
        "TripwireTess",
    ];

    // Every fifth line is the server talking. Denser than a real server, because a scene is
    // held only as long as it takes to frame a shot and a log of pure conversation never shows
    // that system lines render differently.
    const EVENT_EVERY: u64 = 5;

    // Out of ten events. Deaths lead because they are the recognisable ones, but not so far
    // that the log reads as a slaughter — which is what a 6-in-10 weighting produced.
    const DEATH_SHARE: u64 = 4;
    const ADVANCEMENT_SHARE: u64 = 7;

    // One chat line in three is addressed to somebody.
    const DIRECTED_ONE_IN: u64 = 3;

    // Odd multiplier and increment: a full-period generator, and the high bits are the ones
    // used because an LCG's low bits cycle far too short to pick from a list of twenty.
    const MULTIPLIER: u64 = 6_364_136_223_846_793_005;
    const INCREMENT: u64 = 1_442_695_040_888_963_407;

    pub fn new(names: Vec<String>, seed: u64) -> Self {
        let count = names.len();
        Self {
            names,
            state: seed,
            beat: 0,
            speakers: Deck::new(count),
            targets: Deck::new(count),
            subjects: Deck::new(count),
            lines: Deck::new(Self::LINES.len()),
            directed: Deck::new(Self::DIRECTED.len()),
            deaths: Deck::new(Self::DEATHS.len()),
            advancements: Deck::new(Self::ADVANCEMENTS.len()),
            comings: Deck::new(Self::COMINGS.len()),
            visitors: Deck::new(Self::VISITORS.len()),
        }
    }

    /// The next line, or `None` when there is nobody to attribute one to.
    pub fn next(&mut self) -> Option<ChatBeat> {
        if self.names.is_empty() {
            return None;
        }

        self.beat += 1;
        if self.beat % Self::EVENT_EVERY == 0 {
            return Some(ChatBeat::Event { text: self.event() });
        }

        let author_index = Self::draw(&mut self.speakers, &mut self.state);
        let author = self.names[author_index].clone();

        let directed = self.names.len() > 1
            && Self::roll(&mut self.state, Self::DIRECTED_ONE_IN) == 0;

        let text = if directed {
            let to = self.other_than(author_index);
            let template = Self::DIRECTED[Self::draw(&mut self.directed, &mut self.state)];
            template.replace("{to}", &to)
        } else {
            Self::LINES[Self::draw(&mut self.lines, &mut self.state)].to_string()
        };

        Some(ChatBeat::Chat { author, text })
    }

    fn event(&mut self) -> String {
        let roll = Self::roll(&mut self.state, 10);

        // Deaths and advancements belong to the people in the scene: both are things that
        // happen to a player who then carries on talking. Comings belong to visitors.
        let (template, who) = if roll < Self::DEATH_SHARE {
            (
                Self::DEATHS[Self::draw(&mut self.deaths, &mut self.state)],
                self.names[Self::draw(&mut self.subjects, &mut self.state)].clone(),
            )
        } else if roll < Self::ADVANCEMENT_SHARE {
            (
                Self::ADVANCEMENTS[Self::draw(&mut self.advancements, &mut self.state)],
                self.names[Self::draw(&mut self.subjects, &mut self.state)].clone(),
            )
        } else {
            (
                Self::COMINGS[Self::draw(&mut self.comings, &mut self.state)],
                Self::VISITORS[Self::draw(&mut self.visitors, &mut self.state)].to_string(),
            )
        };

        template.replace("{who}", &who)
    }

    // A name from the target deck that is not the speaker. Dealing again rather than picking
    // freshly keeps the deck's coverage guarantee; at worst it costs one extra deal.
    fn other_than(&mut self, author_index: usize) -> String {
        for _ in 0..self.names.len() {
            let index = Self::draw(&mut self.targets, &mut self.state);
            if index != author_index {
                return self.names[index].clone();
            }
        }

        // Unreachable with two or more names, and returning the speaker is better than looping.
        self.names[author_index].clone()
    }

    fn draw(deck: &mut Deck, state: &mut u64) -> usize {
        if deck.is_empty() {
            return 0;
        }
        let mut roll = |modulo: u64| Self::roll(state, modulo);
        deck.deal(&mut roll)
    }

    fn roll(state: &mut u64, modulo: u64) -> u64 {
        *state = state
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::INCREMENT);
        if modulo == 0 {
            return 0;
        }
        (*state >> 33) % modulo
    }
}
