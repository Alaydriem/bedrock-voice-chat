import { prefersReducedMotion, registerPainter, rgba, type PaintContext } from './engine';
import { PLAYERS } from './players';

/**
 * The multitrack view: one scrolling lane per speaker, each lane in that
 * player's colour, with a playhead pinned to the right edge.
 *
 * This is the capability nothing else in the category has, so it gets a
 * visual rather than a bullet.
 */

/** A deterministic speech envelope: bursts of talking with gaps between. */
function envelope(lane: number, i: number): number {
  const a = Math.sin(i * 0.21 + lane * 2.1);
  const b = Math.sin(i * 0.071 + lane * 4.7);
  const c = Math.sin(i * 0.53 + lane * 1.3);
  const level = Math.abs(a * 0.5 + b * 0.3 + c * 0.2);
  const gate = Math.sin(i * 0.029 + lane * 2.7);
  return gate > 0.05 ? Math.min(1, level * 1.5 * Math.min(1, gate * 2.4)) : 0;
}

registerPainter('tracks', ({ ctx, w, h, t }: PaintContext) => {
  const pad = 12;
  const lanes = PLAYERS.length;
  const laneH = (h - pad * 2) / lanes;
  const cell = 4;
  const gap = 2;
  const pitch = cell + gap;
  const rowsMax = Math.max(2, Math.floor((laneH - 20) / 2 / pitch));
  const scroll = prefersReducedMotion() ? 0 : t * 0.032;

  for (let lane = 0; lane < lanes; lane++) {
    const player = PLAYERS[lane]!;
    const cy = pad + laneH * lane + laneH / 2 + 6;

    ctx.strokeStyle = 'rgba(148,131,182,.16)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(pad, cy);
    ctx.lineTo(w - pad, cy);
    ctx.stroke();

    for (let px = pad; px < w - pad; px += pitch) {
      const idx = Math.floor((px + scroll) / pitch);
      const n = Math.round(envelope(lane, idx) * rowsMax);
      if (n <= 0) {
        ctx.fillStyle = rgba(player.hue, 0.18);
        ctx.fillRect(px, cy - cell / 2, cell, cell);
        continue;
      }
      ctx.fillStyle = player.hue;
      for (let k = -n; k <= n; k++) {
        ctx.fillRect(px, cy + k * pitch - cell / 2, cell, cell);
      }
    }

    ctx.font = '10px ui-monospace, Menlo, monospace';
    ctx.fillStyle = 'rgba(251,248,255,.85)';
    ctx.fillText(player.name, pad + 2, pad + laneH * lane + 12);
  }

  ctx.fillStyle = 'rgba(255,106,77,.85)';
  ctx.fillRect(w - pad - 2, pad, 2, h - pad * 2);
});
