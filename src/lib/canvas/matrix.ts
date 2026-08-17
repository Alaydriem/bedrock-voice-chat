import { registerPainter, type PaintContext } from './engine';

/**
 * Where BVC runs.
 *
 * Support is three dots, not a waveform. A waveform here read as a level meter,
 * which implied a *measurement* of compatibility rather than a state of it —
 * and a level meter that sits still is a broken level meter.
 *
 * The palette deliberately stops before the warm end of the spectrum. Red at
 * the far end of a support scale reads as "broken" no matter what the label
 * says, and nothing in this table is broken. Both states are drawn from the
 * cool half — teal through green for full support, blue through cyan for the
 * client-side path — so the difference reads as *which route*, not as
 * better-and-worse.
 *
 * `proxy` marks Realms, where no server-side addon is possible and the client
 * handles positioning instead.
 */

/** Cool-half stops only. No red, no amber. */
const DOTS_FULL = ['#21d8d8', '#34d8a0', '#3bd869'] as const;
const DOTS_PROXY = ['#466cf3', '#3d93ed', '#28bae1'] as const;
const DOT_OFF = 'rgba(90,72,130,.45)';

type Support = 'full' | 'proxy';

interface Row {
  readonly label: string;
  readonly support: Support;
}

interface Group {
  readonly title: string;
  readonly rows: readonly Row[];
}

const GROUPS: readonly Group[] = [
  {
    title: 'MINECRAFT BEDROCK',
    rows: [
      { label: 'Bedrock Dedicated Server', support: 'full' },
      { label: 'Aternos and other hosts', support: 'full' },
      { label: 'Realms', support: 'full' },
      { label: 'Console, via the mobile app', support: 'full' }
    ],
  },
  {
    title: 'MINECRAFT JAVA',
    rows: [
      { label: 'Fabric', support: 'full' },
      { label: 'Paper', support: 'full' }
    ],
  },
  {
    title: 'INTEGRATIONS',
    rows: [
      { label: 'Rich API', support: 'full' },
      { label: 'Simple Voice Chat', support: 'full' }
    ],
  },
];

const HEADER_H = 30;
const ROW_H = 27;
const TAG: Record<Support, string> = { full: 'SUPPORTED', proxy: 'VIA CLIENT' };

registerPainter('matrix', ({ ctx, w, h }: PaintContext) => {
  const pad = Math.max(14, w * 0.05);
  const heights = GROUPS.map((g) => HEADER_H + 14 + g.rows.length * ROW_H + 8);
  const total = heights.reduce((a, b) => a + b, 0) + 12 * (GROUPS.length - 1);

  // The table is laid out in fixed pixels, but the canvas is sized by its CSS
  // box. Adding a group grows the table and nothing grows the box, so without
  // this the last group paints past the bottom edge and overflow:hidden eats
  // it. Shrink to fit instead: a crowded table is legible, a missing row is
  // not. The box also carries a min-height, so this only bites on the narrow
  // end of the two-column range.
  const fit = Math.min(1, (h - pad * 2) / total);
  ctx.save();
  ctx.translate((w * (1 - fit)) / 2, (h - total * fit) / 2);
  ctx.scale(fit, fit);
  let y = 0;

  const dot = 7;
  const dotGap = 3;
  const dotsW = 3 * dot + 2 * dotGap;

  GROUPS.forEach((group, gi) => {
    const boxH = heights[gi]!;

    ctx.fillStyle = 'rgba(28,17,50,.55)';
    ctx.strokeStyle = '#432f6c';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.roundRect(pad, y, w - pad * 2, boxH, 9);
    ctx.fill();
    ctx.stroke();

    ctx.font = '10px ui-monospace, Menlo, monospace';
    ctx.fillStyle = '#b3a4d0';
    ctx.fillText(group.title, pad + 14, y + 21);

    ctx.strokeStyle = '#432f6c';
    ctx.beginPath();
    ctx.moveTo(pad + 14, y + HEADER_H);
    ctx.lineTo(w - pad - 14, y + HEADER_H);
    ctx.stroke();

    group.rows.forEach((row, ri) => {
      const ry = y + HEADER_H + 22 + ri * ROW_H;
      const full = row.support === 'full';
      const ramp = full ? DOTS_FULL : DOTS_PROXY;

      // Three dots. Full support lights all three; the client-side route lights
      // two, so it reads as a shorter path rather than a worse one.
      const lit = full ? 3 : 2;
      for (let k = 0; k < 3; k++) {
        ctx.fillStyle = k < lit ? ramp[k]! : DOT_OFF;
        ctx.fillRect(pad + 14 + k * (dot + dotGap), ry - 7, dot, dot);
      }

      ctx.font = '12px ui-sans-serif, system-ui, sans-serif';
      ctx.fillStyle = '#d6cbea';
      ctx.fillText(row.label, pad + 14 + dotsW + 16, ry);

      const tag = TAG[row.support];
      ctx.font = '10px ui-monospace, Menlo, monospace';
      ctx.fillStyle = full ? '#3bd869' : '#28bae1';
      ctx.fillText(tag, w - pad - 14 - ctx.measureText(tag).width, ry);
    });

    y += boxH + 12;
  });

  ctx.restore();
});
