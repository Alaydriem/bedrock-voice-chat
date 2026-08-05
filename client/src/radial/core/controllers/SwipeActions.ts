/**
 * Swipe-to-reveal, as arithmetic.
 *
 * A row sits over a tray of actions. Whether a gesture is a drag or a tap, how far the row has
 * travelled, and whether the tray stays out when the finger lifts are all decisions about
 * numbers, so they live here where a test can reach them — the gesture itself needs a
 * touchscreen, and everything that can be settled without one is.
 *
 * The component owns the pointer events and the transform. This owns what they mean.
 */
export class SwipeActions {
  /** Under this much travel the gesture is a tap, and the row's own click stands. */
  static readonly SLOP = 8;

  /** Past this share of the tray's width, lifting the finger leaves it open. */
  static readonly LATCH = 0.4;

  /** How far past the tray the row may be dragged, so the end of the travel has a feel. */
  static readonly OVERSHOOT = 24;

  static isSwipe(dx: number): boolean {
    return Math.abs(dx) >= SwipeActions.SLOP;
  }

  /**
   * Where the row sits for a given drag.
   *
   * Leftward only. The tray is on the right, so dragging right would expose the panel behind
   * the row, which holds nothing — clamped at zero rather than rubber-banded, because there is
   * no content there to hint at.
   */
  static offset(dx: number, trayWidth: number, wasOpen: boolean): number {
    const from = wasOpen ? -trayWidth : 0;
    const raw = from + dx;
    if (raw > 0) return 0;
    return Math.max(-(trayWidth + SwipeActions.OVERSHOOT), raw);
  }

  /** Whether the tray stays open once the pointer lifts. */
  static latches(offset: number, trayWidth: number): boolean {
    if (trayWidth <= 0) return false;
    return Math.abs(offset) >= trayWidth * SwipeActions.LATCH;
  }

  /** Where the row rests: against the tray when open, flush when closed. */
  static resting(open: boolean, trayWidth: number): number {
    return open ? -trayWidth : 0;
  }
}
