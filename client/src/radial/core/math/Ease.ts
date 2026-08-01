/** Interpolation and easing shared by every renderer and every sequence. */
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

  static inOutCubic(t: number): number {
    return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
  }

  static outBack(t: number, k = 1.34): number {
    return 1 + (k + 1) * Math.pow(t - 1, 3) + k * Math.pow(t - 1, 2);
  }

  /** Progress through a window, from its start time and duration. */
  static phase(t: number, start: number, duration: number): number {
    return Ease.clamp01((t - start) / duration);
  }
}
