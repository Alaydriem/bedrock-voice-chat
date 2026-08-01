/**
 * The boot sequence, as durations in seconds at speed 1.
 *
 * The mark writes on flat, takes its colours, dances; the ring collapses inward
 * onto it; the two beat together; the mark settles. What it is saying is that the
 * ring and the mark are one object, which is the relationship the whole product
 * relies on and which nobody has to be told if they watch it happen once.
 */
export const INTRO_PHASES = {
  /** The mark writes on, left to right, collapsed flat, in idle grey. */
  scan: 1.3,
  /** Blocks take their spectrum colours. */
  charge: 0.3,
  /** Amplitude ramps up: the mark starts dancing. */
  wave: 1.4,
  /** The 72 ring bars fly inward onto their resting radius. */
  implode: 0.9,
  /** Ring and mark beat together. */
  pulse: 1.7,
  /** The mark collapses back to its mid row, still in colour. */
  settle: 0.9,
} as const;

export type IntroPhase = keyof typeof INTRO_PHASES;

/** Cumulative end time of each phase, in seconds at speed 1. */
export class IntroMarks {
  readonly scan: number;
  readonly charge: number;
  readonly wave: number;
  readonly implode: number;
  readonly pulse: number;
  readonly settle: number;
  /** Length of the whole sequence, excluding any hold. */
  readonly total: number;

  private constructor() {
    let acc = 0;
    this.scan = acc += INTRO_PHASES.scan;
    this.charge = acc += INTRO_PHASES.charge;
    this.wave = acc += INTRO_PHASES.wave;
    this.implode = acc += INTRO_PHASES.implode;
    this.pulse = acc += INTRO_PHASES.pulse;
    this.settle = acc += INTRO_PHASES.settle;
    this.total = this.settle;
  }

  static readonly AT = new IntroMarks();
}
