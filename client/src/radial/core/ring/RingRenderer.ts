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
  /**
   * Drop the time term from the bar profile, keeping its shape around the circle. The
   * ring then holds a fixed silhouette that `rot` sweeps round, instead of writhing in
   * place — a ring that spins without drawing attention as a second moving thing.
   *
   * Distinct from `reduce`, which flattens the profile altogether. A flat ring cannot
   * appear to rotate at all: every bar is identical and evenly spaced, so each angle looks
   * like every other.
   */
  still?: boolean;
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

  static readonly TWO_PI = Math.PI * 2;

  /**
   * Beyond this angular distance a source's gaussian weight is under 1e-8, which no
   * amplitude or colour decision can see. Skipping those bars is what keeps the cost
   * of a source proportional to the arc it occupies rather than to the whole ring.
   */
  static readonly CUTOFF = RingRenderer.SIGMA * 6;

  /**
   * Per-segment alpha, indexed by segment. A stack's fade is a function of the segment
   * index alone, so it is the same table on every bar of every frame.
   *
   * A loud bar can ask for more segments than `SEG`. The fade has already reached its
   * 0.3 floor by the last entry, so clamping the index is not an approximation of the
   * formula past the end of the table — it is the same number.
   */
  static readonly SEGMENT_ALPHA: readonly number[] = Array.from(
    { length: RingGeometry.SEG + 1 },
    (_, k) => (k === 0 ? 0.62 : Math.max(0.3, 1 - k / (RingGeometry.SEG + 2))),
  );

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
    const denom = 2 * RingRenderer.SIGMA * RingRenderer.SIGMA;

    for (let b = 0; b < BARS; b++) {
      const alpha = paint.alphaFor ? paint.alphaFor(b) : 1;
      if (alpha <= 0) continue;

      const angle = -Math.PI / 2 + b * step + rot;
      let amp = hum + boost;
      if (!paint.reduce) {
        // `angle` already carries `rot`, so a still profile rotates rigidly with the ring.
        const phase = paint.still ? 0 : t;
        amp +=
          0.05 * Math.sin(angle * 3 + phase * 0.0011) +
          0.035 * Math.sin(angle * 7 - phase * 0.0015);
      }

      let color = base;
      for (const source of sources) {
        // Wrapped to [-PI, PI] by subtracting whole turns. The atan2(sin, cos) form
        // this replaces cost three trig calls per bar per source — with eight voices
        // on a phone that was over a thousand per frame, for a value a subtraction
        // and a round produce exactly.
        const raw = angle - source.angle;
        const delta = raw - RingRenderer.TWO_PI * Math.round(raw / RingRenderer.TWO_PI);
        if (delta > RingRenderer.CUTOFF || delta < -RingRenderer.CUTOFF) continue;
        const weight = Math.exp(-(delta * delta) / denom);
        if (weight > 0.22) color = Color.mix(base, source.hue, Math.min(1, weight * 1.3));
        amp = Math.max(amp, hum + source.volume * weight * 0.86);
      }

      const offset = paint.offsetFor ? paint.offsetFor(b) : 0;
      const lineWidth = g.pitch * 0.66 * (paint.widthFor ? paint.widthFor(b) : 1);
      const segments = Math.max(0, Math.round(amp * SEG));

      // Colour is set once per bar and the segment fade rides globalAlpha, which
      // multiplies identically to baking it into the colour. Setting strokeStyle per
      // segment instead meant building and parsing a colour string up to ten times a
      // bar — the ring's dominant cost, and the reason it stuttered on a phone.
      x.strokeStyle = color;
      x.lineWidth = lineWidth;
      for (let k = 0; k <= segments; k++) {
        x.globalAlpha = RingRenderer.SEGMENT_ALPHA[k < SEG ? k : SEG] * alpha;
        x.beginPath();
        x.arc(g.cx, g.cy, g.R + offset + k * g.pitch, angle - arcWidth / 2, angle + arcWidth / 2);
        x.stroke();
      }
      x.globalAlpha = 1;
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
