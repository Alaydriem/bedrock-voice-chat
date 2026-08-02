import { MarkData } from '../radial/MarkData';
import { MarkRenderer } from '../radial/MarkRenderer';
import { RingGeometry, RingRenderer, type RingSource } from '../radial/RingRenderer';
import { prefersReducedMotion, registerPainter, type PaintContext } from './engine';
import { PLAYERS, playerAt } from './players';

/**
 * The radial. You are the mark at the centre; everyone audible returns a
 * coloured ping on the bearing they are standing on, and the return shortens as
 * they walk away.
 *
 * This is the client's idle visual, drawn by the client's own renderer, so a
 * visitor who downloads recognises the first screen they see.
 *
 * <canvas data-cv="ring">
 */
registerPainter('ring', ({ ctx, w, h, t }: PaintContext) => {
  const reduce = prefersReducedMotion();
  const geometry = RingGeometry.fit(w, h);

  const sources: RingSource[] = [];
  for (const p of PLAYERS) {
    const s = playerAt(p, t);
    if (s.volume > 0.03) sources.push({ angle: s.bearing, volume: s.volume, hue: p.hue });
  }

  RingRenderer.draw(ctx, { geometry, t, sources, reduce });

  // The mark sits inside the ring: you are the logo, everyone else is a return.
  const cell = Math.max(2, Math.floor((geometry.inner * 1.46) / MarkData.COLS));
  const gap = MarkData.gapFor(cell);
  const markW = MarkData.width(cell, gap);
  const markH = MarkData.height(cell, gap);

  MarkRenderer.draw(ctx, {
    ox: geometry.cx - markW / 2,
    oy: geometry.cy - markH / 2,
    cell,
    gap,
    t,
    mortarAlpha: 0.9,
    reduce,
  });
});
