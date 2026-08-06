/**
 * A canvas and its backing store.
 *
 * Device pixel ratio is clamped to 2. Beyond that the mark's blocks gain no
 * definition — they are axis-aligned rectangles — and the ring's 72 bars times 9
 * segments starts costing real time on a phone.
 */
export class Surface {
  static readonly MAX_DPR = 2;

  readonly canvas: HTMLCanvasElement;
  readonly ctx: CanvasRenderingContext2D;

  #width = 0;
  #height = 0;
  #dpr = 1;

  /** The last box taken from layout, or null when the next frame has to ask again. */
  #measured: { width: number; height: number } | null = null;
  #observer: ResizeObserver | null = null;

  constructor(canvas: HTMLCanvasElement) {
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("radial: 2D context unavailable");
    this.canvas = canvas;
    this.ctx = ctx;
    this.#watch();
  }

  /** Width in CSS px. */
  get width(): number {
    return this.#width;
  }

  /** Height in CSS px. */
  get height(): number {
    return this.#height;
  }

  get dpr(): number {
    return this.#dpr;
  }

  static dprNow(): number {
    const raw = typeof window === "undefined" ? 1 : window.devicePixelRatio || 1;
    return Math.min(raw, Surface.MAX_DPR);
  }

  /**
   * Take the size from the element's layout box. Returns false when the canvas
   * has no box yet — inside a hidden pane, before first layout — which is the
   * caller's signal to skip the frame rather than paint into a 0x0 store.
   *
   * The box is measured once and then remembered until a resize says otherwise.
   * Reading it is what turns a pending style change anywhere on the page into a
   * synchronous reflow, and a binding calls this every frame: the boot preloader
   * writes a braille glyph eleven times a second beneath its ring, and measuring
   * afterwards charged the ring a forced layout for each one, on the launch that
   * is already the busiest moment the app has.
   */
  fit(): boolean {
    if (this.#measured) {
      this.#apply(this.#measured.width, this.#measured.height, false);
      return true;
    }

    const rect = this.canvas.getBoundingClientRect();
    // A canvas with no box is not remembered. Caching that answer would leave the
    // surface permanently convinced it can never paint, and the pane it is waiting
    // on may open without ever resizing the canvas itself.
    if (!rect.width || !rect.height) return false;

    if (this.#observer) {
      this.#measured = { width: rect.width, height: rect.height };
    }
    this.#apply(rect.width, rect.height, false);
    return true;
  }

  /** Set an explicit size in CSS px and write it to the element's style. */
  resize(width: number, height: number): void {
    // Recorded as the known box: this size came from the caller rather than from
    // layout, and a later frame must not measure its way back to the old one.
    if (this.#observer) this.#measured = { width, height };
    this.#apply(width, height, true);
  }

  /** Release the resize subscription. Called from the owning binding's `destroy`. */
  destroy(): void {
    this.#observer?.disconnect();
    this.#observer = null;
    this.#measured = null;
  }

  /** Reset the transform, clear, and hand back the context ready to paint. */
  begin(): CanvasRenderingContext2D {
    const { ctx } = this;
    ctx.setTransform(this.#dpr, 0, 0, this.#dpr, 0, 0);
    ctx.clearRect(0, 0, this.#width, this.#height);
    return ctx;
  }

  /**
   * Watch the canvas so a real size change forgets the remembered box.
   *
   * Without a `ResizeObserver` — server-side rendering, a test environment — there is
   * nothing to invalidate the cache, so nothing is cached and every frame measures. That
   * is the behaviour this class had throughout, and it stays correct where it is the only
   * option available.
   */
  #watch(): void {
    if (typeof ResizeObserver === "undefined") return;
    this.#observer = new ResizeObserver(() => {
      this.#measured = null;
    });
    this.#observer.observe(this.canvas);
  }

  #apply(width: number, height: number, writeStyle: boolean): void {
    const dpr = Surface.dprNow();
    const w = Math.round(width * dpr);
    const h = Math.round(height * dpr);
    // Both dimensions are compared. Checking width alone leaves a height-only
    // change — a pane growing taller — rendering into a stale backing store.
    if (this.canvas.width !== w || this.canvas.height !== h) {
      this.canvas.width = w;
      this.canvas.height = h;
    }
    if (writeStyle) {
      this.canvas.style.width = `${width}px`;
      this.canvas.style.height = `${height}px`;
    }
    this.#width = width;
    this.#height = height;
    this.#dpr = dpr;
  }
}
