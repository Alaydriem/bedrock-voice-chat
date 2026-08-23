/// Deals from a shuffled deck, reshuffling only once it is empty.
///
/// Independent draws were the first attempt and read wrong: picking from ten templates eight
/// times collides about as often as not, so a short log showed the same line three times and
/// killed the same player twice. Dealing guarantees everything is used once before anything
/// repeats, which is what makes a two-minute log look like a server rather than a loop.
pub struct Deck {
    order: Vec<usize>,
    dealt: usize,
}

impl Deck {
    pub fn new(size: usize) -> Self {
        Self {
            order: (0..size).collect(),
            dealt: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// The next index, reshuffling when the deck runs out.
    ///
    /// `roll` supplies randomness rather than owning it, so one generator drives every deck in
    /// a script and the whole sequence stays reproducible from a single seed.
    pub fn deal(&mut self, roll: &mut impl FnMut(u64) -> u64) -> usize {
        if self.order.is_empty() {
            return 0;
        }

        if self.dealt >= self.order.len() {
            self.shuffle(roll);
            self.dealt = 0;
        }

        let index = self.order[self.dealt];
        self.dealt += 1;
        index
    }

    // Fisher-Yates, and it deliberately may leave the first card equal to the last card dealt.
    // Forbidding that would bias every reshuffle; at these list sizes one adjacent repeat every
    // full cycle is rarer than the collisions this replaced.
    fn shuffle(&mut self, roll: &mut impl FnMut(u64) -> u64) {
        for i in (1..self.order.len()).rev() {
            let j = roll((i + 1) as u64) as usize;
            self.order.swap(i, j);
        }
    }
}
