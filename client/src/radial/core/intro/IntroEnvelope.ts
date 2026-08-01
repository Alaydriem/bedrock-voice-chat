import { Ease } from "../math/Ease";
import type { IntroEndState } from "./IntroConfig";
import { INTRO_PHASES, IntroMarks } from "./IntroPhases";

const AT = IntroMarks.AT;

/**
 * The amplitude and beat envelopes of the boot sequence, as pure functions of time.
 *
 * Separate from IntroSequence because these are the part worth checking and they have
 * nothing to do with a canvas: a test can assert that the mark is flat before the
 * charge phase, that the collapse lands on the configured rest amplitude, and that
 * three beats happen during the pulse, without constructing a 2D context.
 */
export class IntroEnvelope {
  readonly endState: IntroEndState;
  readonly idleGain: number;

  constructor(endState: IntroEndState = "flat", idleGain = 0.32) {
    this.endState = endState;
    this.idleGain = idleGain;
  }

  /** Where the mark rests once the sequence has landed. */
  get restGain(): number {
    return this.endState === "dance" ? this.idleGain : 0;
  }

  /** Three decaying hits through the pulse phase, fading out across the settle. */
  beat(t: number): number {
    if (t < AT.implode) return 0;
    const p = Ease.clamp01((t - AT.implode) / INTRO_PHASES.pulse);
    const local = (p * 3) % 1;
    const decay = 1 - Ease.clamp01((t - AT.pulse) / INTRO_PHASES.settle);
    return Math.pow(1 - local, 2.4) * (p < 1 ? 1 : decay);
  }

  /** Mark amplitude across the whole timeline. */
  gain(t: number): number {
    // Nothing has happened yet: the mark is writing on, flat and grey.
    if (t < AT.charge) return 0;
    // Rising into the dance.
    if (t < AT.wave) return Ease.outCubic(Ease.phase(t, AT.charge, INTRO_PHASES.wave * 0.55));
    // Easing back to make room for the ring arriving.
    if (t < AT.implode) {
      return Ease.lerp(1, 0.55, Ease.inOutCubic(Ease.phase(t, AT.wave, INTRO_PHASES.implode)));
    }
    const beating = 0.55 + this.beat(t) * 0.45;
    if (t < AT.pulse) return beating;
    return Ease.lerp(beating, this.restGain, Ease.inOutCubic(Ease.phase(t, AT.pulse, INTRO_PHASES.settle)));
  }

  /** Amplitude during a loader collapse, which overrides the timeline entirely. */
  collapseGain(progress: number): number {
    return Ease.lerp(this.idleGain, 0, Ease.inOutCubic(Ease.clamp01(progress)));
  }
}
