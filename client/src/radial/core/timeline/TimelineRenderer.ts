import { Color } from "../color/Color";
import { TimelineEnvelope } from "./TimelineEnvelope";

export interface TimelineLane {
  name: string;
  hue: string;
}

export interface TimelinePaint {
  width: number;
  height: number;
  lanes: readonly TimelineLane[];
  /** Elapsed time in milliseconds; drives the scroll. */
  t: number;
  cell?: number;
  gap?: number;
  /** Sample provider, so a real recording can replace the synthetic envelope. */
  envelope?: (lane: number, index: number) => number;
  reduce?: boolean;
}

/**
 * Multitrack recording, one lane per player.
 *
 * Blocks rather than a smooth waveform, at the same pitch as the mark. A track is
 * the same substance as a voice meter, seen over minutes instead of milliseconds.
 */
export class TimelineRenderer {
  static draw(x: CanvasRenderingContext2D, paint: TimelinePaint): void {
    const { width: w, height: h, lanes, t } = paint;
    const cell = paint.cell ?? 5;
    const gap = paint.gap ?? 2;
    const pitch = cell + gap;
    const envelope = paint.envelope ?? TimelineEnvelope.at;
    const laneHeight = h / lanes.length;
    const rowsMax = Math.max(2, Math.floor((laneHeight - 22) / 2 / pitch));
    const scroll = paint.reduce ? 0 : t * 0.032;

    lanes.forEach((lane, index) => {
      const cy = laneHeight * index + laneHeight / 2 + 7;

      x.strokeStyle = "rgba(148,131,182,.16)";
      x.lineWidth = 1;
      x.beginPath();
      x.moveTo(0, cy);
      x.lineTo(w, cy);
      x.stroke();

      for (let px = 0; px < w; px += pitch) {
        const i = Math.floor((px + scroll) / pitch);
        const rows = Math.round(envelope(index, i) * rowsMax);
        if (rows <= 0) {
          x.fillStyle = Color.rgba(lane.hue, 0.18);
          x.fillRect(px, cy - cell / 2, cell, cell);
          continue;
        }
        x.fillStyle = lane.hue;
        for (let k = -rows; k <= rows; k++) {
          x.fillRect(px, cy + k * pitch - cell / 2, cell, cell);
        }
      }

      x.font = "11px ui-monospace, Menlo, monospace";
      x.fillStyle = "rgba(251,248,255,.85)";
      x.fillText(lane.name, 4, laneHeight * index + 13);
    });

    // Playhead: recording is happening at the right edge, now.
    x.fillStyle = "rgba(255,106,77,.85)";
    x.fillRect(w - 2, 0, 2, h);
  }
}
