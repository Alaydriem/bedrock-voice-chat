import { prefersReducedMotion, registerPainter, rgba, type PaintContext } from './engine';

/**
 * Top-down proximity map. You are the white square; everyone inside the dashed
 * ring can hear you, and the ripple around each person is how loud they are
 * from where you are standing.
 *
 * Ambient — it plays itself. An earlier draft made this draggable and the
 * interaction got in the way of the message, so the people move instead of
 * the viewer.
 *
 * Available in the kit; not part of the default composition.
 * <canvas data-cv="scene">
 */

interface Actor {
  readonly name: string;
  readonly hue: string;
  /** Centre of this actor's wander, 0..1 of the box. */
  readonly cx: number;
  readonly cy: number;
  readonly radius: number;
  readonly speed: number;
  readonly phase: number;
}

const ACTORS: readonly Actor[] = [
  { name: 'ALAYDRIEM', hue: '#21d8d8', cx: 0.3, cy: 0.32, radius: 0.1, speed: 0.00022, phase: 0 },
  { name: 'PETRA', hue: '#aee236', cx: 0.72, cy: 0.26, radius: 0.13, speed: -0.00017, phase: 1.4 },
  { name: 'JUNO', hue: '#f67414', cx: 0.68, cy: 0.74, radius: 0.09, speed: 0.00025, phase: 2.8 },
  { name: 'KESTREL', hue: '#6a50e9', cx: 0.24, cy: 0.72, radius: 0.12, speed: -0.00013, phase: 4.1 },
];

registerPainter('scene', ({ ctx, w, h, t }: PaintContext) => {
  const still = prefersReducedMotion();
  const shortest = Math.min(w, h);
  const reach = shortest * 0.34;
  const mx = w / 2;
  const my = h / 2;

  const grid = Math.max(28, shortest / 14);
  ctx.strokeStyle = 'rgba(148,131,182,.07)';
  ctx.lineWidth = 1;
  for (let gx = grid; gx < w; gx += grid) {
    ctx.beginPath();
    ctx.moveTo(gx, 0);
    ctx.lineTo(gx, h);
    ctx.stroke();
  }
  for (let gy = grid; gy < h; gy += grid) {
    ctx.beginPath();
    ctx.moveTo(0, gy);
    ctx.lineTo(w, gy);
    ctx.stroke();
  }

  for (let r = 1; r <= 3; r++) {
    ctx.beginPath();
    ctx.arc(mx, my, (reach * r) / 3, 0, Math.PI * 2);
    ctx.strokeStyle = rgba('#bb8dfa', 0.06 + 0.08 * ((3 - r) / 3));
    ctx.lineWidth = 1;
    ctx.stroke();
  }

  ctx.beginPath();
  ctx.arc(mx, my, reach, 0, Math.PI * 2);
  ctx.strokeStyle = rgba('#bb8dfa', 0.34);
  ctx.lineWidth = 1.5;
  ctx.setLineDash([5, 6]);
  ctx.stroke();
  ctx.setLineDash([]);

  for (const a of ACTORS) {
    const angle = still ? a.phase : a.phase + t * a.speed;
    const px = (a.cx + Math.cos(angle) * a.radius) * w;
    const py = (a.cy + Math.sin(angle) * a.radius) * h;
    const dist = Math.hypot(px - mx, py - my);
    const volume = Math.max(0, 1 - dist / reach);

    if (volume > 0.02) {
      ctx.beginPath();
      ctx.moveTo(mx, my);
      ctx.lineTo(px, py);
      ctx.strokeStyle = rgba(a.hue, 0.1 + volume * 0.35);
      ctx.lineWidth = 1;
      ctx.stroke();
    }

    const level = still ? volume : volume * (0.55 + 0.45 * Math.abs(Math.sin(t * 0.0021 + a.phase)));
    if (level > 0.02) {
      for (let k = 1; k <= 4; k++) {
        ctx.beginPath();
        ctx.arc(px, py, 10 + k * 9 * level, 0, Math.PI * 2);
        ctx.strokeStyle = rgba(a.hue, Math.max(0, (0.4 - k * 0.08) * level));
        ctx.lineWidth = 2;
        ctx.stroke();
      }
    }

    ctx.fillStyle = volume > 0.02 ? a.hue : 'rgba(120,100,160,.5)';
    ctx.fillRect(px - 6, py - 6, 12, 12);

    ctx.font = '10px ui-monospace, Menlo, monospace';
    ctx.fillStyle = volume > 0.02 ? 'rgba(251,248,255,.9)' : 'rgba(179,164,208,.45)';
    ctx.fillText(a.name, px - ctx.measureText(a.name).width / 2, py - 15);
  }

  ctx.fillStyle = '#fbf8ff';
  ctx.fillRect(mx - 7, my - 7, 14, 14);
  ctx.fillStyle = '#1c1132';
  ctx.fillRect(mx - 3, my - 3, 6, 6);
  ctx.font = '10px ui-monospace, Menlo, monospace';
  ctx.fillStyle = 'rgba(251,248,255,.75)';
  ctx.fillText('YOU', mx - ctx.measureText('YOU').width / 2, my + 26);
});
