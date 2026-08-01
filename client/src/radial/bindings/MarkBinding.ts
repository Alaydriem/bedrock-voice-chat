import { AnimationLoop } from "../core/canvas/AnimationLoop";
import { Surface } from "../core/canvas/Surface";
import { Visibility } from "../core/canvas/Visibility";
import { MarkData } from "../core/mark/MarkData";
import { MarkRenderer } from "../core/mark/MarkRenderer";
import { type Binding, CssNumber } from "./Binding";

export interface MarkOptions {
  /** Block size in CSS px. Overridden by `--rad-mark-cell` when that is set. */
  cell?: number;
  /** Space between blocks. Defaults to 30% of the cell. */
  gap?: number;
  /** A hex colour, or `rainbow` for the spectrum. */
  color?: string | "rainbow";
  mortar?: boolean;
  mortarColor?: string;
  /** Amplitude, 0 to 1. */
  gain?: number;
  /** Hold amplitude still rather than dancing. */
  still?: boolean;
  /** Pad the canvas by half a gap so mortar is not clipped at the edges. */
  bleed?: boolean;
  loop?: AnimationLoop;
}

/**
 * The mark, dancing.
 *
 *   <canvas data-rad-mark data-cell="6"></canvas>
 *
 * Cell size is read from the `--rad-mark-cell` custom property when present, so a
 * container query decides how big the mark is and the binding never needs to know
 * what a phone is. Re-read on resize, not per frame.
 */
export class MarkBinding implements Binding {
  readonly canvas: HTMLCanvasElement;

  #surface: Surface;
  #options: MarkOptions;
  #cell = 6;
  #gap = 2;
  #stop: (() => void) | null = null;
  #observer: ResizeObserver | null = null;
  #reduce = Visibility.prefersReducedMotion();

  constructor(canvas: HTMLCanvasElement, options: MarkOptions = {}) {
    this.canvas = canvas;
    this.#surface = new Surface(canvas);
    this.#options = options;
    this.#measure();

    if (typeof ResizeObserver !== "undefined") {
      const host = canvas.parentElement ?? canvas;
      this.#observer = new ResizeObserver(() => this.#measure());
      this.#observer.observe(host);
    }

    this.#stop = (options.loop ?? AnimationLoop.shared()).add((t) => this.#paint(t));
  }

  /** Amplitude, 0 to 1. Set this from a level source to make the mark a meter. */
  set gain(value: number) {
    this.#options.gain = value;
  }

  get gain(): number {
    return this.#options.gain ?? 1;
  }

  set color(value: string | "rainbow" | undefined) {
    this.#options.color = value;
  }

  destroy(): void {
    this.#stop?.();
    this.#stop = null;
    this.#observer?.disconnect();
    this.#observer = null;
  }

  #measure(): void {
    const cell = CssNumber.read(this.canvas, "--rad-mark-cell", this.#options.cell ?? 6);
    const gap = CssNumber.read(
      this.canvas,
      "--rad-mark-gap",
      this.#options.gap ?? Math.max(1, Math.round(cell * 0.3)),
    );
    this.#cell = cell;
    this.#gap = gap;
    const bleed = this.#options.bleed === true ? gap : 0;
    this.#surface.resize(MarkData.width(cell, gap) + bleed, MarkData.height(cell, gap) + bleed);
  }

  #paint(t: number): void {
    if (!Visibility.isPaintable(this.canvas)) return;
    const x = this.#surface.begin();
    const bleed = this.#options.bleed === true ? this.#gap / 2 : 0;
    const rainbow = this.#options.color === "rainbow" || this.#options.color === undefined;
    MarkRenderer.draw(x, {
      ox: bleed,
      oy: bleed,
      cell: this.#cell,
      gap: this.#gap,
      t,
      gain: this.#options.gain ?? 1,
      still: this.#options.still,
      tint: rainbow ? null : this.#options.color,
      mortar: this.#options.mortar !== false,
      mortarColor: this.#options.mortarColor,
      reduce: this.#reduce,
    });
  }
}
