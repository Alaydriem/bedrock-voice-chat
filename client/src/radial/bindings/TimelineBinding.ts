import { AnimationLoop } from "../core/canvas/AnimationLoop";
import { Surface } from "../core/canvas/Surface";
import { Visibility } from "../core/canvas/Visibility";
import { type TimelineLane, TimelineRenderer } from "../core/timeline/TimelineRenderer";
import type { Binding } from "./Binding";

export interface TimelineOptions {
  lanes: readonly TimelineLane[];
  cell?: number;
  gap?: number;
  envelope?: (lane: number, index: number) => number;
  loop?: AnimationLoop;
}

/**
 * Multitrack recording.
 *
 *   <div class="rad-timeline-lanes"><canvas data-rad-timeline></canvas></div>
 *
 * Sizes to its container, so a lane area that grows taller gains rows rather than
 * stretching them.
 */
export class TimelineBinding implements Binding {
  readonly canvas: HTMLCanvasElement;

  #surface: Surface;
  #options: TimelineOptions;
  #stop: (() => void) | null = null;
  #reduce = Visibility.prefersReducedMotion();

  constructor(canvas: HTMLCanvasElement, options: TimelineOptions) {
    this.canvas = canvas;
    this.#surface = new Surface(canvas);
    this.#options = options;
    this.#stop = (options.loop ?? AnimationLoop.shared()).add((t) => this.#paint(t));
  }

  setLanes(lanes: readonly TimelineLane[]): void {
    this.#options = { ...this.#options, lanes };
  }

  destroy(): void {
    this.#stop?.();
    this.#stop = null;
  }

  #paint(t: number): void {
    if (!Visibility.isPaintable(this.canvas)) return;
    if (!this.#surface.fit()) return;
    if (this.#options.lanes.length === 0) return;
    const x = this.#surface.begin();
    TimelineRenderer.draw(x, {
      width: this.#surface.width,
      height: this.#surface.height,
      lanes: this.#options.lanes,
      t,
      cell: this.#options.cell,
      gap: this.#options.gap,
      envelope: this.#options.envelope,
      reduce: this.#reduce,
    });
  }
}
