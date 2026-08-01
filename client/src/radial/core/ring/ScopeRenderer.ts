import { Color } from "../color/Color";
import { Ease } from "../math/Ease";
import { MarkRenderer } from "../mark/MarkRenderer";
import { RingGeometry } from "./RingGeometry";
import type { ScopeBuffer } from "./ScopeBuffer";

export interface ScopePaint {
  geometry: RingGeometry;
  buffer: ScopeBuffer;
  /** Elapsed time in milliseconds, for the mark at the centre. */
  t: number;
  /** Value shown in the middle. Eased toward the latest sample by the caller. */
  readout: number;
  readoutUnit?: string;
  /** Sample value mapped to zero amplitude. */
  floor?: number;
  /** Sample range above `floor` mapped to full amplitude. */
  range?: number;
  /** Above this a bar turns warn. */
  warnAt?: number;
  /** Above this a bar turns fault. */
  faultAt?: number;
  reduce?: boolean;
}

/**
 * The ring, re-tasked as an oscilloscope.
 *
 * Same 72 bars, same geometry, same mark in the middle — but each bar is a second
 * of history rather than a direction. Reusing the shape is the point: the thing
 * that means "the system is listening" on the empty state is the thing that means
 * "here is how the link has been behaving", so nobody has to learn a second chart.
 */
export class ScopeRenderer {
  static draw(x: CanvasRenderingContext2D, paint: ScopePaint): void {
    const { geometry: g, buffer, t } = paint;
    const { SEG } = RingGeometry;
    const floor = paint.floor ?? 15;
    const range = paint.range ?? 95;
    const warnAt = paint.warnAt ?? 60;
    const faultAt = paint.faultAt ?? 90;
    const step = (Math.PI * 2) / buffer.size;
    const arcWidth = step * 0.6;

    for (let b = 0; b < buffer.size; b++) {
      const value = buffer.at(b);
      const norm = Ease.clamp((value - floor) / range, 0.06, 1);
      const angle = -Math.PI / 2 + b * step;
      const segments = Math.max(0, Math.round(norm * SEG));
      const color = value > faultAt ? "#ff8266" : value > warnAt ? "#ffcf4d" : "#7bd6b0";
      const life = buffer.life(b);
      const isHead = buffer.age(b) < 2;

      for (let k = 0; k <= segments; k++) {
        x.beginPath();
        x.arc(g.cx, g.cy, g.R + k * g.pitch, angle - arcWidth / 2, angle + arcWidth / 2);
        x.lineWidth = g.pitch * 0.68;
        x.strokeStyle = Color.rgba(
          color,
          isHead ? 0.98 : Math.max(0.2, 0.34 + life * 0.5 - k / (SEG * 2.6)),
        );
        x.stroke();
      }
    }

    x.beginPath();
    x.arc(g.cx, g.cy, g.inner, 0, Math.PI * 2);
    x.strokeStyle = "rgba(148,131,182,.4)";
    x.lineWidth = 1;
    x.stroke();

    const { cell, gap, width, height } = g.markCell(1, 1.3);
    MarkRenderer.draw(x, {
      ox: g.cx - width / 2,
      oy: g.cy - height / 2,
      cell,
      gap,
      t,
      gain: 0.5,
      still: true,
      tint: "#5d4a86",
      mortarColor: "#33224f",
      reduce: paint.reduce,
    });

    x.save();
    x.textAlign = "center";
    x.font = "600 26px ui-monospace, Menlo, monospace";
    x.fillStyle = "#fbf8ff";
    x.fillText(String(Math.round(paint.readout)), g.cx, g.cy + 9);
    x.font = "10px ui-monospace, Menlo, monospace";
    x.fillStyle = "rgba(179,164,208,.9)";
    x.fillText(paint.readoutUnit ?? "MS RTT", g.cx, g.cy + 26);
    x.restore();
  }
}
