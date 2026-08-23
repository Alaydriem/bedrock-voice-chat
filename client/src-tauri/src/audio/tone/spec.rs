/// The parameters one rendered tone needs.
///
/// Held as a spec rather than as constants on each caller so the speaker test and the
/// mute cues share one synthesiser. A second copy of the envelope maths is a second
/// place for a partial to be added without the level being rechecked.
///
/// `Copy` so a caller can hand one back by value from an associated constant. Returning a
/// reference to one instead would rest on const promotion, which is a subtle thing to owe
/// a compile to when every field here is a word wide.
#[derive(Clone, Copy)]
pub struct ToneSpec {
    /// (frequency in Hz, start time in seconds).
    pub notes: &'static [(f32, f32)],

    /// (harmonic multiple, relative amplitude). Falling amplitude with height, as a
    /// struck object behaves; equal partials sound like a buzzer.
    pub partials: &'static [(f32, f32)],

    /// Exponential decay constant.
    pub decay_seconds: f32,

    /// A hard start on a sine is a click.
    pub attack_seconds: f32,

    /// Peak after normalisation.
    pub peak: f32,

    /// Total rendered length. Must leave about three decay constants after the last note
    /// starts, or the render ends on a step and that step is a click.
    pub duration_seconds: f32,
}
