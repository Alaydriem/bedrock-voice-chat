import { Color } from '../radial/Color';
/**
 * One animation frame loop for the whole page.
 *
 * Every visual on the site is a <canvas data-cv="name">. Painters register
 * here by name; the loop finds the canvases, skips the ones that are off
 * screen, and hands each painter a context that is already sized to the
 * device pixel ratio and cleared.
 *
 * One loop rather than one per component means a page with nine visuals still
 * schedules a single rAF callback, and a visual that scrolls out of view stops
 * costing anything.
 */

export interface PaintContext {
  /** Sized to devicePixelRatio and cleared. Draw in CSS pixels. */
  ctx: CanvasRenderingContext2D;
  /** Width in CSS pixels. */
  w: number;
  /** Height in CSS pixels. */
  h: number;
  /** Milliseconds since the loop started. */
  t: number;
  /** The canvas, for reading data- attributes. */
  el: HTMLCanvasElement;
}

export type Painter = (c: PaintContext) => void;

const painters = new Map<string, Painter>();

export function registerPainter(name: string, fn: Painter): void {
  painters.set(name, fn);
}

export const prefersReducedMotion = (): boolean =>
  typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;

/**
 * Size a canvas to its CSS box at the device pixel ratio and clear it.
 * Returns null when the element has no layout yet, so painters can bail.
 */
export function fitToBox(el: HTMLCanvasElement): PaintContext | null {
  const rect = el.getBoundingClientRect();
  if (rect.width < 1 || rect.height < 1) return null;
  return prepare(el, Math.round(rect.width), Math.round(rect.height));
}

/**
 * Size a canvas to an explicit CSS pixel size, setting the style box too.
 * For visuals whose size is derived from their content rather than layout.
 */
export function fitToSize(el: HTMLCanvasElement, w: number, h: number): PaintContext {
  el.style.width = `${w}px`;
  el.style.height = `${h}px`;
  return prepare(el, w, h);
}

function prepare(el: HTMLCanvasElement, w: number, h: number): PaintContext {
  const dpr = Math.min(globalThis.devicePixelRatio || 1, 2);
  const bw = Math.round(w * dpr);
  const bh = Math.round(h * dpr);
  if (el.width !== bw || el.height !== bh) {
    el.width = bw;
    el.height = bh;
  }
  const ctx = el.getContext('2d') as CanvasRenderingContext2D;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, w, h);
  return { ctx, w, h, t: 0, el };
}

/**
 * Colour helpers, delegating to the radial system's Color.
 *
 * These used to be hand-rolled here: a hex-only parser plus a `mix` that
 * returned `rgb(...)`. Feeding one into the other read as NaN and painted every
 * blended bar black. Color.channels accepts both forms, which is why the whole
 * class of bug is gone rather than fixed once — see lib/radial/Color.ts.
 */
export const rgba = (color: string, alpha: number): string => Color.rgba(color, alpha);
export const mix = (from: string, to: string, k: number): string => Color.mix(from, to, k);

let started = false;

/** Start the loop. Safe to call more than once. */
export function startCanvasEngine(): void {
  if (started) return;
  started = true;

  const origin = performance.now();
  const onScreen = new Set<Element>();

  const io = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (e.isIntersecting) onScreen.add(e.target);
        else onScreen.delete(e.target);
      }
    },
    { rootMargin: '160px' }
  );

  const observe = (): void => {
    for (const el of document.querySelectorAll<HTMLCanvasElement>('canvas[data-cv]')) {
      io.observe(el);
    }
  };
  observe();

  // Audience switching swaps which canvases are in the layout.
  document.addEventListener('bvc:audiencechange', observe);

  const frame = (now: number): void => {
    const t = now - origin;
    for (const el of onScreen) {
      const canvas = el as HTMLCanvasElement;
      const painter = painters.get(canvas.dataset.cv ?? '');
      if (!painter) continue;
      const c = fitToBox(canvas);
      if (!c) continue;
      c.t = t;
      painter(c);
    }
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
}
