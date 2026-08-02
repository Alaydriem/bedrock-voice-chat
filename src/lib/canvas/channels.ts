import { MarkData } from '../radial/MarkData';
import { MarkRenderer } from '../radial/MarkRenderer';
import { prefersReducedMotion, registerPainter, type PaintContext } from './engine';
import { PLAYERS } from './players';

/**
 * Two panels: proximity on the left, a group channel on the right. The channel
 * is lit because a channel ignores distance entirely.
 *
 * Each voice is a level meter drawn with the radial system's exact language,
 * matching LevelMeterBinding: cell 3, gap 28% of cell, mortar off (at three
 * pixels a cell the violet reads as mud), tinted to that player's hue while
 * they are above the speech threshold and to the idle violet below it.
 *
 * The bar beside someone's name is visibly the same object as the mark. That is
 * the whole point of the system — the mark reads as a voice rather than as
 * decoration — so these cannot be generic bar charts.
 */

/** Matches LevelMeterBinding's defaults. */
const CELL = 3;
const GAP = CELL * 0.28;
const THRESHOLD = 0.08;
const IDLE = '#7a68a0';

interface Group {
  readonly title: string;
  /** Indices into PLAYERS. */
  readonly members: readonly number[];
  readonly hot: boolean;
}

const GROUPS: readonly Group[] = [
  { title: 'PROXIMITY', members: [0, 2], hot: false },
  { title: 'SQUAD ALPHA', members: [1], hot: true },
];

/** Speech envelope for a demo voice: bursts with gaps, not a steady sine. */
function levelOf(index: number, t: number, inChannel: boolean): number {
  const speech = Math.max(0, Math.sin(t * 0.0009 + index * 1.7));
  const shape = 0.55 + 0.45 * Math.abs(Math.sin(t * 0.0031 + index * 2.3));
  // A channel member is at full volume regardless of distance; proximity
  // members are attenuated. That contrast is the point of the figure.
  const falloff = inChannel ? 1 : 0.62;
  return Math.min(1, speech * shape * falloff);
}

registerPainter('channels', ({ ctx, w, h, t }: PaintContext) => {
  const reduce = prefersReducedMotion();
  const pad = Math.max(14, w * 0.045);
  const gutter = 10;
  const colW = (w - pad * 2 - gutter) / GROUPS.length;
  const meterW = MarkData.width(CELL, GAP);
  const meterH = MarkData.height(CELL, GAP);

  GROUPS.forEach((group, gi) => {
    const gx = pad + gi * (colW + gutter);
    const gy = pad;
    const gh = h - pad * 2;

    ctx.fillStyle = group.hot ? 'rgba(47,32,84,.72)' : 'rgba(28,17,50,.6)';
    ctx.strokeStyle = group.hot ? '#bb8dfa' : '#432f6c';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.roundRect(gx, gy, colW, gh, 9);
    ctx.fill();
    ctx.stroke();

    ctx.font = '10px ui-monospace, Menlo, monospace';
    ctx.fillStyle = group.hot ? '#bb8dfa' : '#b3a4d0';
    ctx.fillText(group.title, gx + 12, gy + 20);

    ctx.strokeStyle = '#432f6c';
    ctx.beginPath();
    ctx.moveTo(gx + 12, gy + 30);
    ctx.lineTo(gx + colW - 12, gy + 30);
    ctx.stroke();

    group.members.forEach((pi, i) => {
      const player = PLAYERS[pi]!;
      const rowY = gy + 48 + i * 34;
      if (rowY + meterH > gy + gh - 6) return;

      const level = reduce ? 0.55 : levelOf(pi, t, group.hot);
      const speaking = level > THRESHOLD;

      MarkRenderer.draw(ctx, {
        ox: gx + 12,
        oy: rowY,
        cell: CELL,
        gap: GAP,
        t,
        gain: Math.max(0.05, level),
        still: level <= 0.05,
        tint: speaking ? player.hue : IDLE,
        mortar: false,
        reduce,
      });

      ctx.font = '11px ui-monospace, Menlo, monospace';
      ctx.fillStyle = speaking ? '#d6cbea' : '#8d80ad';
      ctx.fillText(player.name, gx + 12 + meterW + 11, rowY + meterH / 2 + 4);
    });
  });
});
