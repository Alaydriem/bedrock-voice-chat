/** What the mark does once the sequence lands. */
export type IntroEndState =
  /** Collapse to the mid row, still in spectrum colour. A finished load. */
  | "flat"
  /** Keep dancing at idle amplitude. A load still in progress. */
  | "dance";

export interface IntroConfig {
  width: number;
  height: number;
  /**
   * Take the size from the canvas's layout box each frame instead of from `width`
   * and `height`. What a sequence embedded in a responsive page wants: the
   * stylesheet owns the size, so a narrow window and a phone both get a mark scaled
   * to the space rather than the fixed box an exported PNG needs.
   */
  fluid: boolean;
  /** Null keeps the alpha channel, which a PNG or a VP9 WebM will preserve. */
  background: string | null;
  /** The colour blocks fade up from during `charge`. */
  idleTint: string;
  /** One colour across every block. Null uses the spectrum. */
  markTint: string | null;
  mortar: boolean;
  mortarColor: string;
  ringColor: string;
  ringIdleColor: string;
  endState: IntroEndState;
  /** Amplitude the mark rests at when `endState` is `dance`. */
  idleGain: number;
  speed: number;
  loop: boolean;
  /** Seconds held on the end state before a loop restarts. */
  holdSeconds: number;
  /** Ring diameter against the canvas. */
  scale: number;
  /** Mark size against the ring. Above 1 it overflows the hairline. */
  logoScale: number;
  glow: boolean;
  /** Ring rotation rate, radians per second. */
  ringSpin: number;
}

export const INTRO_DEFAULTS: IntroConfig = {
  width: 1080,
  height: 1080,
  fluid: false,
  background: "#251844",
  idleTint: "#7a68a0",
  markTint: null,
  mortar: true,
  mortarColor: "#43306e",
  ringColor: "#6a4f96",
  ringIdleColor: "#54407c",
  endState: "flat",
  idleGain: 0.32,
  speed: 1,
  loop: true,
  holdSeconds: 1.6,
  scale: 1,
  logoScale: 1,
  glow: false,
  ringSpin: 0.16,
};

/** Lifted at the moment the ring bars land, and for the bloom. */
export const BRAND_LIFT = "#bb8dfa";
