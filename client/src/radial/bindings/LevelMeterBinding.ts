import { AnimationLoop } from "../core/canvas/AnimationLoop";
import { MeterProbe } from "../core/canvas/MeterProbe";
import { Surface } from "../core/canvas/Surface";
import { RestGate } from "../core/canvas/RestGate";
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
  /**
   * Report received levels and drawn frames to `MeterProbe` under this name.
   *
   * For the meters diagnostics has to be able to vouch for — the self pill has failed both by
   * not receiving and by not drawing what it received, and the two are indistinguishable on
   * screen.
   */
  probe?: string;
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
  /**
   * How long the height takes to reach a louder reading, and to fall back from one.
   *
   * Attack is short because a voice starting has to look immediate; release is longer because
   * the gaps between syllables are shorter than the gaps between messages, and a meter that
   * collapsed into every one of them would flicker rather than read as speech.
   */
  static readonly ATTACK_MS = 70;
  static readonly RELEASE_MS = 260;

  /**
   * At or below this the mark is drawn at its floor, held still.
   *
   * `gain` is clamped up to it and `still` is set from it, so every level under it produces
   * byte-identical pixels — which is what makes skipping the redraw safe rather than merely
   * cheap.
   */
  static readonly REST = 0.05;

  readonly canvas: HTMLCanvasElement;

  #surface: Surface;
  #options: LevelMeterOptions;
  #cell = 3;
  #gap = 0.8;
  #level = 0;
  /**
   * Where the level is heading, as last reported.
   *
   * The mark already dances every frame off the animation clock; what arrives from a source is
   * only its amplitude, and that arrives a couple of times a second rather than sixty. Snapping
   * to it makes a meter that moves continuously but changes height in visible jerks, so the
   * height is eased and the dance carries the frames in between.
   *
   * Nothing here is invented: the target is always a measured level, and a meter told to be
   * still still goes still. Easing only decides how quickly it gets there.
   */
  #target = 0;
  #lastFrame = 0;
  #rest = new RestGate();
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

    if (options.probe) MeterProbe.register(options.probe);
    this.#unsubscribe = options.source?.subscribe((level) => {
      this.#target = level;
      if (options.probe) MeterProbe.level(options.probe, level);
    }) ?? null;

    this.#stop = (options.loop ?? AnimationLoop.shared()).add((t) => this.#paint(t));
  }

  /**
   * Drive the meter directly when there is no LevelSource to subscribe to.
   *
   * Snaps rather than eases. A caller setting this frame by frame is already choosing the
   * shape, and easing on top of that would fight it.
   */
  set level(value: number) {
    const clamped = value < 0 ? 0 : value > 1 ? 1 : value;
    this.#level = clamped;
    this.#target = clamped;
  }

  get level(): number {
    return this.#level;
  }

  set color(value: string | "rainbow") {
    this.#options.color = value;
    // A resting meter has nothing about its level left to change, so without this the new
    // colour would not appear until somebody spoke.
    this.#rest.invalidate();
  }

  /** Replace the source, e.g. when a player card is reused for someone else. */
  setSource(source: LevelSource | null): void {
    this.#unsubscribe?.();
    this.#unsubscribe = source?.subscribe((level) => {
      this.#target = level;
      if (this.#options.probe) MeterProbe.level(this.#options.probe, level);
    }) ?? null;
    if (!source) {
      this.#level = 0;
      this.#target = 0;
    }
    this.#rest.invalidate();
  }

  /**
   * Called more than once for one binding: `LevelMeter` releases it from both its effect's
   * teardown and its `onDestroy`, and either can run first. Everything below is idempotent on
   * its own, but the probe is a count — a second release would take a canvas off the ledger
   * that was never on it.
   */
  destroy(): void {
    if (!this.#stop) return;
    if (this.#options.probe) MeterProbe.release(this.#options.probe);
    this.#stop();
    this.#stop = null;
    this.#unsubscribe?.();
    this.#unsubscribe = null;
    this.#observer?.disconnect();
    this.#observer = null;
    this.#surface.destroy();
  }

  /**
   * Move the drawn height towards the reported one, in real time rather than per frame.
   *
   * Framerate-independent on purpose: a phone dropping to thirty frames a second would
   * otherwise take twice as long to reach the same height, which is exactly the device where
   * the meter is already the least convincing.
   */
  #ease(t: number): void {
    const elapsed = this.#lastFrame ? Math.min(100, t - this.#lastFrame) : 16;
    this.#lastFrame = t;
    if (elapsed <= 0) return;

    const rising = this.#target > this.#level;
    const constant = rising ? LevelMeterBinding.ATTACK_MS : LevelMeterBinding.RELEASE_MS;
    // Exponential approach, so the step is proportional to what is left to cover and the
    // height never overshoots however long a frame took.
    const k = 1 - Math.exp(-elapsed / constant);
    this.#level += (this.#target - this.#level) * k;

    // Settle exactly, or a meter told to be silent keeps drawing a hairline forever and the
    // `still` floor never engages.
    if (Math.abs(this.#target - this.#level) < 0.002) this.#level = this.#target;
  }

  #measure(): void {
    const cell = CssNumber.read(this.canvas, "--rad-meter-cell", this.#options.cell ?? 3);
    const gap = CssNumber.read(this.canvas, "--rad-meter-gap", this.#options.gap ?? cell * 0.28);
    this.#cell = cell;
    this.#gap = gap;
    this.#surface.resize(MarkData.width(cell, gap), MarkData.height(cell, gap));
    // A resize clears the canvas, so the resting picture no longer exists.
    this.#rest.invalidate();
  }

  #paint(t: number): void {
    this.#ease(t);

    // Asked before `isPaintable`, which reads `offsetWidth` and forces layout. A quiet roster
    // is mostly resting meters, so this is the check that keeps them from costing a reflow
    // each, every frame, to decide whether to redraw pixels that would not change.
    const atRest = this.#level <= LevelMeterBinding.REST && this.#target <= LevelMeterBinding.REST;
    if (!this.#rest.needsPaint(atRest)) return;

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
    this.#rest.painted(atRest);
    // Only frames above the floor count: the ledger measures whether a voice moved the meter,
    // and the one resting repaint after silence would read as a paint that never happened.
    if (this.#options.probe && !atRest) MeterProbe.painted(this.#options.probe);
  }
}
