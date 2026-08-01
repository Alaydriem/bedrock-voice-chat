import { Color } from "../color/Color";
import { RingGeometry } from "./RingGeometry";
import type { RingSource } from "./RingSource";

export interface RingPaint {
  geometry: RingGeometry;
  /** Elapsed time in milliseconds. */
  t: number;
  /** Bar colour before any source tints it. */
  base?: string;
  /** Resting amplitude. 0.15 alive, 0.07 at rest. */
  hum?: number;
  /** Amplitude added to every bar, for a beat. */
  boost?: number;
  /** Rotation in radians. */
  rot?: number;
  /** Voices to place around the circle. */
  sources?: readonly RingSource[];
  /** Draw the hairline circle just inside the bars. */
  hairline?: boolean;
  hairlineColor?: string;
  /** Per-bar radial offset, for the implosion. */
  offsetFor?: (bar: number) => number;
  /** Per-bar alpha, for the implosion. */
  alphaFor?: (bar: number) => number;
  /** Per-bar width multiplier, for the implosion. */
  widthFor?: (bar: number) => number;
  reduce?: boolean;
}

/**
 * The ring: 72 bars, each a radial stack of arc segments.
 *
 * The ring is the empty state and the status oscilloscope. It is never a control
 * surface and never a roster — identification wants recognition, not recall, and a
 * circle makes you find a mark, work out whose it is, then find the control. Names
 * and controls live in the list.
 */
export class RingRenderer {
  /** Angular spread of a source's influence, in radians. */
  static readonly SIGMA = 0.3;

  static draw(x: CanvasRenderingContext2D, paint: RingPaint): void {
    const { geometry: g, t } = paint;
    const { BARS, SEG } = RingGeometry;
    const step = (Math.PI * 2) / BARS;
    const arcWidth = step * 0.58;
    const base = paint.base ?? "#6a4f96";
    const hum = paint.hum ?? 0.15;
    const boost = paint.boost ?? 0;
    const rot = paint.rot ?? 0;
    const sources = paint.sources ?? [];

    for (let b = 0; b < BARS; b++) {
      const alpha = paint.alphaFor ? paint.alphaFor(b) : 1;
      if (alpha <= 0) continue;

      const angle = -Math.PI / 2 + b * step + rot;
      let amp = hum + boost;
      if (!paint.reduce) {
        amp += 0.05 * Math.sin(angle * 3 + t * 0.0011) + 0.035 * Math.sin(angle * 7 - t * 0.0015);
      }

      let color = base;
      for (const source of sources) {
        const delta = Math.atan2(Math.sin(angle - source.angle), Math.cos(angle - source.angle));
        const weight = Math.exp(-(delta * delta) / (2 * RingRenderer.SIGMA * RingRenderer.SIGMA));
        if (weight > 0.22) color = Color.mix(base, source.hue, Math.min(1, weight * 1.3));
        amp = Math.max(amp, hum + source.volume * weight * 0.86);
      }

      const offset = paint.offsetFor ? paint.offsetFor(b) : 0;
      const lineWidth = g.pitch * 0.66 * (paint.widthFor ? paint.widthFor(b) : 1);
      const segments = Math.max(0, Math.round(amp * SEG));

      for (let k = 0; k <= segments; k++) {
        x.beginPath();
        x.arc(g.cx, g.cy, g.R + offset + k * g.pitch, angle - arcWidth / 2, angle + arcWidth / 2);
        x.lineWidth = lineWidth;
        x.strokeStyle = Color.rgba(color, (k === 0 ? 0.62 : Math.max(0.3, 1 - k / (SEG + 2))) * alpha);
        x.stroke();
      }
    }

    if (paint.hairline !== false) {
      x.beginPath();
      x.arc(g.cx, g.cy, g.inner, 0, Math.PI * 2);
      x.strokeStyle = paint.hairlineColor ?? "rgba(148,131,182,.42)";
      x.lineWidth = 1;
      x.stroke();
    }
  }
}
