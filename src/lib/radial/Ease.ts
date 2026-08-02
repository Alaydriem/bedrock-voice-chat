/**
 * Interpolation and easing shared by every renderer.
 *
 * Ported from client/src/radial/core/math/Ease.ts. The radial system in the
 * client is the source of truth for all of this; the site renders the same
 * marks and the same ring, so it runs the same code rather than a lookalike.
 */
export class Ease {
  static clamp01(v: number): number {
    return v < 0 ? 0 : v > 1 ? 1 : v;
  }

  static clamp(v: number, min: number, max: number): number {
    return v < min ? min : v > max ? max : v;
  }

  static lerp(a: number, b: number, t: number): number {
    return a + (b - a) * t;
  }

  static outCubic(t: number): number {
    return 1 - Math.pow(1 - t, 3);
  }

  /** Progress through a window, from its start time and duration. */
  static phase(t: number, start: number, duration: number): number {
    return Ease.clamp01((t - start) / duration);
  }
}
