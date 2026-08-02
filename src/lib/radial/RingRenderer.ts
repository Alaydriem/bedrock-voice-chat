import { Color } from './Color';

/**
 * Ring geometry, derived from the box it has to fit.
 * Ported from client/src/radial/core/ring/RingGeometry.ts.
 */
export class RingGeometry {
  static readonly BARS = 72;
  static readonly SEG = 8;

  readonly cx: number;
  readonly cy: number;
  readonly R: number;
  readonly pitch: number;
  /** Radius of the hairline just inside the bars. */
  readonly inner: number;

  private constructor(cx: number, cy: number, R: number, pitch: number) {
    this.cx = cx;
    this.cy = cy;
    this.R = R;
    this.pitch = pitch;
    this.inner = R - pitch * 0.9;
  }

  static fit(w: number, h: number): RingGeometry {
    const span = Math.min(w, h) * 0.84;
    const pitch = span / 2 / (RingGeometry.SEG + 7);
    return new RingGeometry(w / 2, h / 2, pitch * 7, pitch);
  }
}

/** A voice placed around the circle. */
export interface RingSource {
  /** Bearing in radians. */
  readonly angle: number;
  /** 0 out of range, 1 right next to you. */
  readonly volume: number;
  readonly hue: string;
}

export interface RingPaint {
  geometry: RingGeometry;
  t: number;
  /** Bar colour before any source tints it. */
  base?: string;
  /** Resting amplitude. 0.15 alive, 0.07 at rest. */
  hum?: number;
  boost?: number;
  rot?: number;
  sources?: readonly RingSource[];
  hairline?: boolean;
  hairlineColor?: string;
  reduce?: boolean;
}

/**
 * The ring: 72 bars, each a radial stack of arc segments.
 *
 * Ported from client/src/radial/core/ring/RingRenderer.ts. A source tints the
 * bars near its bearing and raises their amplitude, so a voice reads as a
 * coloured return coming in on a heading — the radar ping.
 */
export class RingRenderer {
  /** Angular spread of a source's influence, in radians. */
  static readonly SIGMA = 0.3;

  static draw(x: CanvasRenderingContext2D, paint: RingPaint): void {
    const { geometry: g, t } = paint;
    const { BARS, SEG } = RingGeometry;
    const step = (Math.PI * 2) / BARS;
    const arcWidth = step * 0.58;
    const base = paint.base ?? '#6a4f96';
    const hum = paint.hum ?? 0.15;
    const boost = paint.boost ?? 0;
    const rot = paint.rot ?? 0;
    const sources = paint.sources ?? [];

    for (let b = 0; b < BARS; b++) {
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

      const segments = Math.max(0, Math.round(amp * SEG));
      for (let k = 0; k <= segments; k++) {
        x.beginPath();
        x.arc(g.cx, g.cy, g.R + k * g.pitch, angle - arcWidth / 2, angle + arcWidth / 2);
        x.lineWidth = g.pitch * 0.66;
        x.strokeStyle = Color.rgba(color, k === 0 ? 0.62 : Math.max(0.3, 1 - k / (SEG + 2)));
        x.stroke();
      }
    }

    if (paint.hairline !== false) {
      x.beginPath();
      x.arc(g.cx, g.cy, g.inner, 0, Math.PI * 2);
      x.strokeStyle = paint.hairlineColor ?? 'rgba(148,131,182,.42)';
      x.lineWidth = 1;
      x.stroke();
    }
  }
}
