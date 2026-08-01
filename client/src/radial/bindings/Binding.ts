/**
 * Anything mounted onto an element and later torn down.
 *
 * Every binding owns a loop registration, and some own a subscription as well.
 * Leaking either keeps a canvas painting after its screen is gone, so `destroy`
 * is not optional and Mount tracks it for you.
 */
export interface Binding {
  destroy(): void;
}

/** Read a numeric CSS custom property off an element, for container-query-driven sizing. */
export class CssNumber {
  static read(el: Element, property: string, fallback: number): number {
    const raw = getComputedStyle(el).getPropertyValue(property).trim();
    if (!raw) return fallback;
    const n = Number.parseFloat(raw);
    return Number.isFinite(n) ? n : fallback;
  }
}
