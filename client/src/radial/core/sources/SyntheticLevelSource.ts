import { AnimationLoop } from "../canvas/AnimationLoop";
import { Visibility } from "../canvas/Visibility";
import type { LevelListener, LevelSource, Unsubscribe } from "./LevelSource";

export interface SyntheticOptions {
  /** Offsets this voice from the others so a roster does not pulse in unison. */
  phase?: number;
  /** Ceiling, before falloff. */
  peak?: number;
  /** Fraction of the cycle spent silent. Speech has gaps. */
  silence?: number;
  /** Scales the whole output. Distance falloff and per-player gain go here. */
  gain?: number;
  loop?: AnimationLoop;
}

/**
 * Speech, simulated.
 *
 * Every reference page runs on this, and it is also the stand-in when a real
 * source exists but `prefers-reduced-motion` is set. Two beating sines with a
 * silence gate: enough to read as a person talking rather than as a VU meter on a
 * test tone.
 */
export class SyntheticLevelSource implements LevelSource {
  #listeners = new Set<LevelListener>();
  #level = 0;
  #stop: Unsubscribe | null = null;
  #loop: AnimationLoop;
  #phase: number;
  #peak: number;
  #silence: number;

  gain: number;

  constructor(options: SyntheticOptions = {}) {
    this.#phase = options.phase ?? 0;
    this.#peak = options.peak ?? 1;
    this.#silence = options.silence ?? 0;
    this.gain = options.gain ?? 1;
    this.#loop = options.loop ?? AnimationLoop.shared();
  }

  get level(): number {
    return this.#level;
  }

  subscribe(listener: LevelListener): Unsubscribe {
    this.#listeners.add(listener);
    listener(this.#level);
    this.#ensureRunning();
    return () => {
      this.#listeners.delete(listener);
      if (this.#listeners.size === 0) this.close();
    };
  }

  close(): void {
    this.#stop?.();
    this.#stop = null;
    this.#listeners.clear();
  }

  /** The envelope at a time, in milliseconds. Pure, so a scrubber can drive it. */
  at(t: number): number {
    if (Visibility.prefersReducedMotion()) return this.#peak * 0.4 * this.gain;
    const speech = Math.max(this.#silence, Math.sin(t * 0.0009 + this.#phase)) - this.#silence;
    const shimmer = 0.55 + 0.45 * Math.abs(Math.sin(t * 0.0031 + this.#phase));
    const v = Math.max(0, speech) * shimmer * this.#peak * this.gain;
    return v > 1 ? 1 : v;
  }

  #ensureRunning(): void {
    if (this.#stop) return;
    this.#stop = this.#loop.add((t) => {
      this.#level = this.at(t);
      for (const listener of this.#listeners) listener(this.#level);
    });
  }
}
