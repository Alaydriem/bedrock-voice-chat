import { MarkData } from "../mark/MarkData";

/**
 * Where the ring sits inside its canvas, and how big the mark at its centre is.
 *
 * Derived rather than configured: every ring in the product — the empty state, the
 * gate options, the sign-in lock, the boot sequence, the scope — uses this, so
 * they are all the same object at different sizes.
 */
export class RingGeometry {
  /** Radial segments stacked outward per bar. */
  static readonly SEG = 8;
  /** Bars around the circle. */
  static readonly BARS = 72;

  readonly cx: number;
  readonly cy: number;
  /** Radius of the innermost segment of a bar. */
  readonly R: number;
  /** Radial distance between segments; also drives bar width. */
  readonly pitch: number;
  /** Radius of the hairline circle, just inside the bars. */
  readonly inner: number;
  /** The diameter the whole assembly is fitted to. */
  readonly span: number;

  private constructor(cx: number, cy: number, R: number, pitch: number, span: number) {
    this.cx = cx;
    this.cy = cy;
    this.R = R;
    this.pitch = pitch;
    this.inner = R - pitch * 0.9;
    this.span = span;
  }

  /**
   * @param fill fraction of the shorter canvas axis the ring occupies. 0.84 is
   *   the system's ratio; the scope uses 0.9 and the dashboard idle ring 0.86.
   */
  static fit(width: number, height: number, scale = 1, fill = 0.84): RingGeometry {
    const span = Math.min(width, height) * fill * scale;
    const pitch = span / 2 / (RingGeometry.SEG + 7);
    return new RingGeometry(width / 2, height / 2, pitch * 7, pitch, span);
  }

  /**
   * Cell and gap for the mark that sits inside the hairline.
   * @param logoScale 1 is the system's own ratio against the ring. Above 1 the
   *   mark overflows the hairline circle, which is a deliberate option.
   * @param reach how far across the inner circle the mark spans. 1.46 for the
   *   full-strength ring, 1.3 for the scope, where a numeric readout shares the
   *   middle.
   */
  markCell(logoScale = 1, reach = 1.46): { cell: number; gap: number; width: number; height: number } {
    const cell = Math.max(2, Math.floor(((this.inner * reach) / MarkData.COLS) * logoScale));
    const gap = Math.max(1, Math.round(cell * 0.34));
    return { cell, gap, width: MarkData.width(cell, gap), height: MarkData.height(cell, gap) };
  }
}
