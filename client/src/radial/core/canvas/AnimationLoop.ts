export type Frame = (t: number) => void;

/**
 * One requestAnimationFrame loop for the whole kit.
 *
 * A page can hold a header mark, a ring, a dozen level meters, group meters and a
 * scope at once. Each of those owning its own rAF loop means each one also owns a
 * DOM query per frame, and the browser interleaves them unpredictably so meters
 * that should share a phase drift apart. One loop, one timestamp, one pass.
 */
export class AnimationLoop {
  static #shared: AnimationLoop | null = null;

  #frames = new Set<Frame>();
  #raf = 0;
  #running = false;

  /** The loop every renderer joins unless a test needs its own. */
  static shared(): AnimationLoop {
    AnimationLoop.#shared ??= new AnimationLoop();
    return AnimationLoop.#shared;
  }

  get size(): number {
    return this.#frames.size;
  }

  get isRunning(): boolean {
    return this.#running;
  }

  /** Register a per-frame callback. Returns the function that unregisters it. */
  add(frame: Frame): () => void {
    this.#frames.add(frame);
    this.start();
    return () => this.remove(frame);
  }

  remove(frame: Frame): void {
    this.#frames.delete(frame);
    if (this.#frames.size === 0) this.stop();
  }

  start(): void {
    if (this.#running || this.#frames.size === 0) return;
    this.#running = true;
    const tick = (t: number) => {
      for (const frame of this.#frames) frame(t);
      if (this.#running) this.#raf = requestAnimationFrame(tick);
    };
    this.#raf = requestAnimationFrame(tick);
  }

  stop(): void {
    this.#running = false;
    if (this.#raf) cancelAnimationFrame(this.#raf);
    this.#raf = 0;
  }

  /** Drive one frame by hand. For tests and for scrubbers. */
  step(t: number): void {
    for (const frame of this.#frames) frame(t);
  }
}
