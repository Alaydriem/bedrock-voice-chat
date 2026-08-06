import { AnimationLoop } from "../core/canvas/AnimationLoop";
import { Surface } from "../core/canvas/Surface";
import { Visibility } from "../core/canvas/Visibility";
import { MarkData } from "../core/mark/MarkData";
import { MarkRenderer } from "../core/mark/MarkRenderer";
import type { LevelSource, Unsubscribe } from "../core/sources/LevelSource";
import { type Binding, CssNumber } from "./Binding";

export interface LevelMeterOptions {
  /** Where the level comes from. Without one the meter sits at its floor. */
  source?: LevelSource;
  /** A hex colour, or `rainbow` for the spectrum. */
  color?: string | "rainbow";
  /** Colour below `threshold`: the meter is present but nobody is talking. */
  idleColor?: string;
  /** Level below which the meter reads as silent. */
  threshold?: number;
  cell?: number;
  gap?: number;
  /** Called when the level crosses `liveAt`, for a card's live styling. */
  onLive?: (live: boolean) => void;
  liveAt?: number;
  loop?: AnimationLoop;
}

/**
 * A voice, as a level meter.
 *
 *   <canvas data-rad-level data-color="#21d8d8"></canvas>
 *
 * The same routine as the header logo at low amplitude, which is the point: the
 * bar beside someone's name is visibly the same object as the mark, so the mark
 * reads as a voice rather than as decoration.
 *
 * Mortar is off. At three pixels a cell the violet reads as mud rather than as
 * substance behind the blocks.
 */
export class LevelMeterBinding implements Binding {
  readonly canvas: HTMLCanvasElement;

  #surface: Surface;
  #options: LevelMeterOptions;
  #cell = 3;
  #gap = 0.8;
  #level = 0;
  #live = false;
  #stop: (() => void) | null = null;
  #unsubscribe: Unsubscribe | null = null;
  #observer: ResizeObserver | null = null;
  #reduce = Visibility.prefersReducedMotion();

  constructor(canvas: HTMLCanvasElement, options: LevelMeterOptions = {}) {
    this.canvas = canvas;
    this.#surface = new Surface(canvas);
    this.#options = options;
    this.#measure();

    if (typeof ResizeObserver !== "undefined") {
      this.#observer = new ResizeObserver(() => this.#measure());
      this.#observer.observe(canvas.parentElement ?? canvas);
    }

    this.#unsubscribe = options.source?.subscribe((level) => {
      this.#level = level;
    }) ?? null;

    this.#stop = (options.loop ?? AnimationLoop.shared()).add((t) => this.#paint(t));
  }

  /** Drive the meter directly when there is no LevelSource to subscribe to. */
  set level(value: number) {
    this.#level = value < 0 ? 0 : value > 1 ? 1 : value;
  }

  get level(): number {
    return this.#level;
  }

  set color(value: string | "rainbow") {
    this.#options.color = value;
  }

  /** Replace the source, e.g. when a player card is reused for someone else. */
  setSource(source: LevelSource | null): void {
    this.#unsubscribe?.();
    this.#unsubscribe = source?.subscribe((level) => {
      this.#level = level;
    }) ?? null;
    if (!source) this.#level = 0;
  }

  destroy(): void {
    this.#stop?.();
    this.#stop = null;
    this.#unsubscribe?.();
    this.#unsubscribe = null;
    this.#observer?.disconnect();
    this.#observer = null;
    this.#surface.destroy();
  }

  #measure(): void {
    const cell = CssNumber.read(this.canvas, "--rad-meter-cell", this.#options.cell ?? 3);
    const gap = CssNumber.read(this.canvas, "--rad-meter-gap", this.#options.gap ?? cell * 0.28);
    this.#cell = cell;
    this.#gap = gap;
    this.#surface.resize(MarkData.width(cell, gap), MarkData.height(cell, gap));
  }

  #paint(t: number): void {
    if (!Visibility.isPaintable(this.canvas)) return;

    const threshold = this.#options.threshold ?? 0.08;
    const liveAt = this.#options.liveAt ?? 0.14;
    const live = this.#level > liveAt;
    if (live !== this.#live) {
      this.#live = live;
      this.#options.onLive?.(live);
    }

    const speaking = this.#level > threshold;
    const rainbow = this.#options.color === "rainbow" || this.#options.color === undefined;
    const tint = speaking ? (rainbow ? null : (this.#options.color as string)) : (this.#options.idleColor ?? "#7a68a0");

    const x = this.#surface.begin();
    MarkRenderer.draw(x, {
      ox: 0,
      oy: 0,
      cell: this.#cell,
      gap: this.#gap,
      t,
      gain: Math.max(0.05, this.#level),
      still: this.#level <= 0.05,
      tint,
      mortar: false,
      reduce: this.#reduce,
    });
  }
}
