import { MarkData } from '../radial/MarkData';
import { MarkRenderer } from '../radial/MarkRenderer';
import { prefersReducedMotion, registerPainter, type PaintContext } from './engine';

/**
 * Painter: fills its own box with the mark, dancing.
 *
 * Dancing is the default. A still mark is a logo; a dancing one is a waveform,
 * and the waveform is the whole idea — the mark is the level meter is the ring.
 * Pass data-still="1" only where motion would be noise.
 *
 * <canvas data-cv="mark" data-still="1" data-gain="0.6">
 */
registerPainter('mark', ({ ctx, w, h, t, el }: PaintContext) => {
  const cell = Math.min(
    MarkData.cellForWidth(w),
    h / (MarkData.ROWS + 0.3 * (MarkData.ROWS - 1))
  );
  const gap = cell * 0.3;
  const markW = MarkData.width(cell, gap);
  const markH = MarkData.height(cell, gap);

  MarkRenderer.draw(ctx, {
    ox: (w - markW) / 2,
    oy: (h - markH) / 2,
    cell,
    gap,
    t,
    gain: Number(el.dataset.gain ?? 1),
    still: el.dataset.still === '1',
    mortar: el.dataset.mortar !== '0',
    reduce: prefersReducedMotion(),
  });
});

/**
 * Painter: the mark stretched edge to edge as a spectrum band, each column
 * fading downward. A full-bleed rule between sections.
 *
 * <canvas data-cv="band">
 */
registerPainter('band', ({ ctx, w, h, t }: PaintContext) => {
  const reduce = prefersReducedMotion();
  const gap = Math.max(2, (w / MarkData.COLS) * 0.14);
  const cell = (w - gap * (MarkData.COLS - 1)) / MarkData.COLS;
  const rowH = h / MarkData.ROWS;

  for (let c = 0; c < MarkData.COLS; c++) {
    const col = MarkData.COLUMNS[c]!;
    const amp = MarkRenderer.dance(c, t, 1, reduce);
    const [top, bottom] = MarkRenderer.extent(c, Math.max(0.05, amp));
    const y0 = top * rowH;
    const y1 = (bottom + 1) * rowH;

    const grad = ctx.createLinearGradient(0, y0, 0, y1);
    grad.addColorStop(0, col[2]);
    grad.addColorStop(1, `${col[2]}6b`);
    ctx.fillStyle = grad;
    ctx.fillRect(c * (cell + gap), y0, cell, y1 - y0);
  }
});
