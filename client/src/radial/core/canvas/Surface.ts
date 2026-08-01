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

  constructor(canvas: HTMLCanvasElement) {
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("radial: 2D context unavailable");
    this.canvas = canvas;
    this.ctx = ctx;
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
   */
  fit(): boolean {
    const rect = this.canvas.getBoundingClientRect();
    if (!rect.width || !rect.height) return false;
    this.#apply(rect.width, rect.height, false);
    return true;
  }

  /** Set an explicit size in CSS px and write it to the element's style. */
  resize(width: number, height: number): void {
    this.#apply(width, height, true);
  }

  /** Reset the transform, clear, and hand back the context ready to paint. */
  begin(): CanvasRenderingContext2D {
    const { ctx } = this;
    ctx.setTransform(this.#dpr, 0, 0, this.#dpr, 0, 0);
    ctx.clearRect(0, 0, this.#width, this.#height);
    return ctx;
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
