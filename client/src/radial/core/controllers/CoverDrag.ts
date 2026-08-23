/**
 * Drag a cover down to dismiss it, as arithmetic.
 *
 * The component owns the pointer events and the transform. This owns the numbers.
 */
export class CoverDrag {
  /** Under this much travel the gesture is a tap or the start of a scroll. */
  static readonly SLOP = 10;

  /** Travel past this, and lifting the finger dismisses. */
  static readonly DISMISS = 110;

  /** Zero: an upward drag has nothing to reveal. */
  static readonly OVERSHOOT_UP = 0;

  static isDrag(dy: number): boolean {
    return dy >= CoverDrag.SLOP;
  }

  /** Downward only, clamped at the resting position. */
  static offset(dy: number): number {
    return Math.max(CoverDrag.OVERSHOOT_UP, dy);
  }

  static dismisses(offset: number): boolean {
    return offset >= CoverDrag.DISMISS;
  }

  /**
   * Whether a drag starting at this scroll position belongs to the cover.
   *
   * iOS reports a negative scrollTop mid-overscroll, hence `<=`.
   */
  static canStart(scrollTop: number): boolean {
    return scrollTop <= 0;
  }
}
