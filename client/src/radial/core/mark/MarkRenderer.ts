import { Color } from "../color/Color";
import { Ease } from "../math/Ease";
import { MarkData } from "./MarkData";

export interface MarkPaint {
  /** Left edge in CSS px. */
  ox: number;
  /** Top edge in CSS px. */
  oy: number;
  /** Block size in CSS px. */
  cell: number;
  /** Space between blocks in CSS px. */
  gap: number;
  /** Elapsed time in milliseconds. The dance is a function of it. */
  t: number;
  /** Amplitude, 0 collapses to the mid row and 1 is the mark at full height. */
  gain?: number;
  /** Amplitude floor, so a silent column is still one block rather than nothing. */
  floor?: number;
  /** Hold the amplitude still instead of dancing. */
  still?: boolean;
  /** Write-on progress, 0 to 1, left to right. */
  reveal?: number;
  /** Crossfade from `idle` to the column's own colour, 0 to 1. */
  tintMix?: number;
  /** The pre-signal colour every block fades up from. */
  idle?: string;
  /** One colour across every block. Null or absent uses the spectrum. */
  tint?: string | null;
  /** Violet behind the blocks. Off for small meters, where it reads as noise. */
  mortar?: boolean;
  mortarColor?: string;
  mortarAlpha?: number;
  /** Skip the dance entirely, for prefers-reduced-motion. */
  reduce?: boolean;
}

/**
 * The mark, drawn linearly.
 *
 * This is the one routine. The header logo, every player level meter, the group
 * meters, the mic meter and the shape at the centre of the ring are all this
 * function at different amplitudes and cell sizes — which is why they read as the
 * same object rather than as a logo plus some bar charts.
 *
 * The violet mortar fills the gaps *inside* the waveform silhouette only. That
 * keeps the blocks reading as blocks without the negative space going black and
 * without a panel appearing around the mark.
 */
export class MarkRenderer {
  /** Per-column amplitude. Two slow sines beating against each other. */
  static dance(col: number, t: number, gain: number, reduce = false): number {
    if (reduce) return gain;
    const a = 0.5 + 0.5 * Math.sin(t * 0.0027 + col * 0.44);
    const b = 0.5 + 0.5 * Math.sin(t * 0.0012 + col * 0.17 + 1.1);
    return gain * (0.42 + 0.58 * (a * 0.62 + b * 0.38));
  }

  /**
   * The rows a column occupies at a given amplitude, as [top, bottom] inclusive.
   * Public because it is the observable contract of the envelope: a test can
   * assert the mark collapses to the mid row at gain 0 without reading pixels.
   */
  static extent(col: number, amplitude: number): readonly [number, number] {
    const [top, bottom] = MarkData.COLUMNS[col];
    const { MID } = MarkData;
    return [
      Math.round(MID - (MID - top) * amplitude),
      Math.round(MID + (bottom - MID) * amplitude),
    ];
  }

  static draw(x: CanvasRenderingContext2D, paint: MarkPaint): void {
    const { ox, oy, cell, gap, t } = paint;
    const gain = paint.gain ?? 1;
    const floor = paint.floor ?? 0.05;
    const reveal = paint.reveal ?? 1;
    const tintMix = paint.tintMix ?? 1;
    const idle = paint.idle ?? "#7a68a0";
    const pitch = cell + gap;
    const cols = MarkData.COLS;

    const extents: (readonly [number, number])[] = [];
    const lit: number[] = [];
    for (let c = 0; c < cols; c++) {
      const raw = paint.still ? gain : MarkRenderer.dance(c, t, gain, paint.reduce);
      extents.push(MarkRenderer.extent(c, Math.max(floor, raw)));
      lit.push(Ease.clamp01(reveal * cols - c));
    }

    if (paint.mortar !== false) {
      const mortarAlpha = paint.mortarAlpha ?? 1;
      const mortarColor = paint.mortarColor ?? "#43306e";
      for (let c = 0; c < cols; c++) {
        if (lit[c] <= 0) continue;
        const [top, bottom] = extents[c];
        x.fillStyle = Color.rgba(mortarColor, mortarAlpha * lit[c]);
        x.fillRect(
          ox + c * pitch - gap / 2,
          oy + top * pitch - gap / 2,
          cell + gap,
          (bottom - top + 1) * pitch,
        );
      }
    }

    for (let c = 0; c < cols; c++) {
      if (lit[c] <= 0) continue;
      const [top, bottom] = extents[c];
      const own = paint.tint ?? MarkData.COLUMNS[c][2];
      const color = tintMix >= 1 ? own : Color.mix(idle, own, tintMix);
      // Blocks land by growing out of their own centre.
      const inset = (1 - lit[c]) * cell * 0.34;
      x.globalAlpha = lit[c];
      x.fillStyle = color;
      for (let r = top; r <= bottom; r++) {
        x.fillRect(ox + c * pitch + inset, oy + r * pitch + inset, cell - inset * 2, cell - inset * 2);
      }
      x.globalAlpha = 1;
    }
  }
}
