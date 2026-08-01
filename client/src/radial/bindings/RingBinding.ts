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
  }

  #paint(t: number): void {
    if (!Visibility.isPaintable(this.canvas)) return;
    if (!this.#surface.fit()) return;

    const o = this.#options;
    const dead = this.mode === "empty";
    const x = this.#surface.begin();
    const g = RingGeometry.fit(this.#surface.width, this.#surface.height, o.scale ?? 1, o.fill ?? 0.84);
    this.#geometry = g;

    RingRenderer.draw(x, {
      geometry: g,
      t,
      sources: dead ? [] : this.#sources,
      hum: dead ? 0.07 : 0.15,
      base: dead ? (o.emptyBase ?? "#54407c") : (o.base ?? "#6a4f96"),
      rot: o.spin ? t * 0.001 * o.spin : 0,
      reduce: this.#reduce,
    });

    const { cell, gap, width, height } = g.markCell(o.logoScale ?? 1);
    MarkRenderer.draw(x, {
      ox: g.cx - width / 2,
      oy: g.cy - height / 2,
      cell,
      gap,
      t,
      gain: dead ? 0.08 : 1,
      still: dead,
      tint: dead ? "#7a68a0" : null,
      mortarColor: dead ? "#2c1f4d" : "#43306e",
      mortarAlpha: 0.9,
      reduce: this.#reduce,
    });
  }
}
