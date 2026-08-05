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

  /**
   * How long without a frame counts as stalled.
   *
   * Generously above one frame at any refresh rate, so an ordinary hitch is never mistaken for
   * a stall.
   */
  static readonly STALL_MS = 1_000;

  #frames = new Set<Frame>();
  #raf = 0;
  #running = false;
  #lastFrameAt = 0;
  #watchdog: ReturnType<typeof setInterval> | null = null;

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
    this.#arm();
    this.#watch();
  }

  #arm(): void {
    this.#lastFrameAt = AnimationLoop.#now();
    const tick = (t: number) => {
      this.#lastFrameAt = AnimationLoop.#now();
      for (const frame of this.#frames) frame(t);
      if (this.#running) this.#raf = requestAnimationFrame(tick);
    };
    this.#raf = requestAnimationFrame(tick);
  }

  /**
   * Re-arm a loop whose frames have stopped arriving.
   *
   * The chain is self-sustaining only while every callback fires: each frame requests the next,
   * so one dropped callback ends the loop for good. That happens when the webview is suspended
   * — minimised, backgrounded, the OS reclaiming it — and the pending request is discarded
   * rather than deferred.
   *
   * Nothing recovered from it. `#running` stays true because `stop` was never called, and
   * `start` refuses to act while `#running`, so the guard that exists to prevent two concurrent
   * loops was also refusing to restart the zero loops that were left. Every meter, ring and
   * scope on the page froze together and stayed frozen until a reload, with no state anywhere
   * indicating anything was wrong.
   *
   * Re-arming while the document is hidden is harmless: the request simply waits for the page to
   * be shown, which is the behaviour that was wanted in the first place.
   */
  #watch(): void {
    if (this.#watchdog !== null || typeof setInterval === "undefined") return;
    this.#watchdog = setInterval(() => {
      if (!this.#running || this.#frames.size === 0) return;
      if (AnimationLoop.#now() - this.#lastFrameAt < AnimationLoop.STALL_MS) return;
      if (this.#raf) cancelAnimationFrame(this.#raf);
      this.#arm();
    }, AnimationLoop.STALL_MS);
  }

  static #now(): number {
    return typeof performance === "undefined" ? Date.now() : performance.now();
  }

  stop(): void {
    this.#running = false;
    if (this.#raf) cancelAnimationFrame(this.#raf);
    this.#raf = 0;
    if (this.#watchdog !== null) {
      clearInterval(this.#watchdog);
      this.#watchdog = null;
    }
  }

  /** Drive one frame by hand. For tests and for scrubbers. */
  step(t: number): void {
    for (const frame of this.#frames) frame(t);
  }
}
