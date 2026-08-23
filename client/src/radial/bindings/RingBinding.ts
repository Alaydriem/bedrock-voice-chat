import { AnimationLoop } from "../core/canvas/AnimationLoop";
import { Surface } from "../core/canvas/Surface";
import { Visibility } from "../core/canvas/Visibility";
import { MarkRenderer } from "../core/mark/MarkRenderer";
import { RingGeometry } from "../core/ring/RingGeometry";
import { RingRenderer } from "../core/ring/RingRenderer";
import type { RingSource } from "../core/ring/RingSource";
import type { Binding } from "./Binding";

export type RingMode =
  /** Voices are speaking. Bars push out and take their hues. */
  | "live"
  /** Nobody is there. Low hum, drained colour, mark held still. */
  | "empty"
  /** One source acquiring — resolving an address, waiting on a handshake. */
  | "lock";

export interface RingOptions {
  mode?: RingMode;
  /** Ring diameter against the canvas. */
  scale?: number;
  /** Mark size against the ring. Above 1 it overflows the hairline. */
  logoScale?: number;
  /** Fraction of the shorter axis the ring occupies. */
  fill?: number;
  /** Rotation rate in radians per second. */
  spin?: number;
  /**
   * Mark amplitude, 0 to 1. Omit and the mode decides — full for a live ring, collapsed
   * for an empty one. Set it when something is actually measuring a level, so the mark
   * rising to its full silhouette is the readout rather than decoration.
   */
  gain?: number;
  /**
   * Hold the bar profile still, so the ring keeps its shape and `spin` sweeps it round
   * rather than the bars moving under their own steam. For a screen where the mark alone
   * is the reading: a border animating independently makes two things move and leaves the
   * viewer to work out which one is answering the question.
   */
  ringStill?: boolean;
  /**
   * An angular window removed from the ring: `[centre, half-width]` in radians. The
   * circle then cannot be completed, which is what a failure is — the gap says a path is
   * broken without a glyph having to say it.
   */
  cut?: readonly [centre: number, half: number];
  /**
   * Colour flared at the two cut ends, as two synthetic sources. Severity lives here
   * rather than on the whole ring: a ring drained to coral reads as an alarm, while two
   * lit ends read as the break itself.
   */
  cutTone?: string;
  /** Paint each bar from the mark's own columns. See `RingPaint.spectrum`. */
  spectrum?: boolean;
  /** Draw the mark at the centre. Off when something else occupies it. */
  mark?: boolean;
  base?: string;
  emptyBase?: string;
  loop?: AnimationLoop;
}

/**
 * The mark with circles around it.
 *
 *   <div class="rad-ring"><canvas data-rad-ring="empty"></canvas></div>
 *
 * In a proximity app you are alone constantly — at connect, while mining, anywhere
 * off the beaten path. That is not an edge case, it is somewhere users live, so the
 * empty state has to say the system is on and listening without asking anyone to
 * interpret it. That is this component's whole job.
 *
 * It is not a control surface and it carries no names. The moment anyone arrives
 * their mark leaves the ring and becomes a card.
 */
export class RingBinding implements Binding {
  readonly canvas: HTMLCanvasElement;

  #surface: Surface;
  #options: RingOptions;
  #sources: readonly RingSource[] = [];
  #geometry: RingGeometry | null = null;
  #stop: (() => void) | null = null;
  #reduce = Visibility.prefersReducedMotion();

  constructor(canvas: HTMLCanvasElement, options: RingOptions = {}) {
    this.canvas = canvas;
    this.#surface = new Surface(canvas);
    this.#options = options;
    this.#stop = (options.loop ?? AnimationLoop.shared()).add((t) => this.#paint(t));
  }

  get mode(): RingMode {
    return this.#options.mode ?? "live";
  }

  set mode(value: RingMode) {
    this.#options.mode = value;
  }

  set gain(value: number | undefined) {
    this.#options.gain = value;
  }

  /**
   * Geometry of the last painted frame, or null before the first.
   * The handoff needs it: a card flies out from where its bar actually was.
   */
  get geometry(): RingGeometry | null {
    return this.#geometry;
  }

  /** Place voices around the circle. Empty is the at-rest ring. */
  setSources(sources: readonly RingSource[]): void {
    this.#sources = sources;
  }

  destroy(): void {
    this.#stop?.();
    this.#stop = null;
    this.#surface.destroy();
  }

  #paint(t: number): void {
    if (!Visibility.isPaintable(this.canvas)) return;
    if (!this.#surface.fit()) return;

    const o = this.#options;
    const dead = this.mode === "empty";
    const x = this.#surface.begin();
    const g = RingGeometry.fit(this.#surface.width, this.#surface.height, o.scale ?? 1, o.fill ?? 0.84);
    this.#geometry = g;

    const cut = o.cut;
    const step = RingRenderer.TWO_PI / RingGeometry.BARS;

    RingRenderer.draw(x, {
      geometry: g,
      t,
      sources: cut ? this.#cutEnds(cut) : dead ? [] : this.#sources,
      hum: dead ? 0.07 : 0.15,
      base: dead ? (o.emptyBase ?? "#54407c") : (o.base ?? "#6a4f96"),
      spectrum: o.spectrum === true,
      rot: o.spin ? t * 0.001 * o.spin : 0,
      still: o.ringStill === true,
      // The gap is measured off the unrotated bar angle, so it stays where it was put
      // even on a ring that spins.
      alphaFor: cut ? (b) => (RingBinding.#inCut(-Math.PI / 2 + b * step, cut) ? 0 : 1) : undefined,
      hairlineArc: cut ? [cut[0] + cut[1], cut[0] - cut[1] + RingRenderer.TWO_PI] : undefined,
      reduce: this.#reduce,
    });

    if (o.mark === false) return;

    const { cell, gap, width, height } = g.markCell(o.logoScale ?? 1);
    MarkRenderer.draw(x, {
      ox: g.cx - width / 2,
      oy: g.cy - height / 2,
      cell,
      gap,
      t,
      gain: o.gain ?? (dead ? 0.08 : 1),
      still: dead,
      tint: dead ? "#7a68a0" : null,
      mortarColor: dead ? "#2c1f4d" : "#43306e",
      mortarAlpha: 0.9,
      reduce: this.#reduce,
    });
  }

  /**
   * Whether a bar falls inside the removed window. Static and exported through
   * `RingBinding.cuts` so the gating can be asserted without a canvas.
   */
  static #inCut(angle: number, [centre, half]: readonly [number, number]): boolean {
    const raw = angle - centre;
    const delta = raw - RingRenderer.TWO_PI * Math.round(raw / RingRenderer.TWO_PI);
    return delta > -half && delta < half;
  }

  /**
   * Bars removed by a cut, as their indices. The observable contract of a severed ring:
   * a contiguous run that never reaches the whole circle, wrapping when the window
   * straddles the ring's zero angle.
   */
  static cuts(cut: readonly [centre: number, half: number]): number[] {
    const step = RingRenderer.TWO_PI / RingGeometry.BARS;
    const bars: number[] = [];
    for (let b = 0; b < RingGeometry.BARS; b++) {
      if (RingBinding.#inCut(-Math.PI / 2 + b * step, cut)) bars.push(b);
    }
    return bars;
  }

  /**
   * The two lit ends of a cut, placed just outside the window so their gaussians fall
   * across the last surviving bars rather than into the gap.
   */
  #cutEnds([centre, half]: readonly [number, number]): readonly RingSource[] {
    const hue = this.#options.cutTone ?? "#ff8266";
    return [
      { angle: centre - half - 0.06, hue, volume: 0.52 },
      { angle: centre + half + 0.06, hue, volume: 0.52 },
    ];
  }
}
