import { AnimationLoop } from "../core/canvas/AnimationLoop";
import { Surface } from "../core/canvas/Surface";
import { Visibility } from "../core/canvas/Visibility";
import { RingGeometry } from "../core/ring/RingGeometry";
import { ScopeBuffer } from "../core/ring/ScopeBuffer";
import { ScopeRenderer } from "../core/ring/ScopeRenderer";
import type { Binding } from "./Binding";

export interface ScopeOptions {
  /** Samples around the circle. One per second of history. */
  samples?: number;
  /** Value every slot starts at, so the scope opens on a plausible trace. */
  fill?: number;
  unit?: string;
  warnAt?: number;
  faultAt?: number;
  loop?: AnimationLoop;
}

/**
 * The ring as an oscilloscope of the last N seconds.
 *
 *   <canvas data-rad-ring="scope"></canvas>
 *
 * The centre readout eases toward the newest sample rather than snapping to it. A
 * number that jitters every second reads as instability even when the link is fine.
 */
export class ScopeBinding implements Binding {
  readonly canvas: HTMLCanvasElement;
  readonly buffer: ScopeBuffer;

  #surface: Surface;
  #options: ScopeOptions;
  #latest: number;
  #shown: number;
  #stop: (() => void) | null = null;
  #reduce = Visibility.prefersReducedMotion();

  constructor(canvas: HTMLCanvasElement, options: ScopeOptions = {}) {
    this.canvas = canvas;
    this.#options = options;
    this.#surface = new Surface(canvas);
    const fill = options.fill ?? 38;
    this.buffer = new ScopeBuffer(options.samples ?? 72, fill);
    this.#latest = fill;
    this.#shown = fill;
    this.#stop = (options.loop ?? AnimationLoop.shared()).add((t) => this.#paint(t));
  }

  /** Record a sample. Call once per tick of whatever is being measured. */
  push(value: number): void {
    this.#latest = value;
    this.buffer.push(value);
  }

  reset(fill?: number): void {
    const v = fill ?? this.#options.fill ?? 38;
    this.buffer.reset(v);
    this.#latest = v;
    this.#shown = v;
  }

  destroy(): void {
    this.#stop?.();
    this.#stop = null;
  }

  #paint(t: number): void {
    if (!Visibility.isPaintable(this.canvas)) return;
    if (!this.#surface.fit()) return;

    this.#shown += (this.#latest - this.#shown) * 0.08;

    const x = this.#surface.begin();
    ScopeRenderer.draw(x, {
      geometry: RingGeometry.fit(this.#surface.width, this.#surface.height, 1, 0.9),
      buffer: this.buffer,
      t,
      readout: this.#shown,
      readoutUnit: this.#options.unit,
      warnAt: this.#options.warnAt,
      faultAt: this.#options.faultAt,
      reduce: this.#reduce,
    });
  }
}
