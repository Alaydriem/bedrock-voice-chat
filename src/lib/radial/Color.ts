import { Ease } from './Ease';

export type Channels = readonly [r: number, g: number, b: number];

/**
 * Colour parsing and blending.
 *
 * Ported from client/src/radial/core/color/Color.ts.
 *
 * `mix` returns hex rather than `rgb()` on purpose. A blended colour is fed
 * straight back into `rgba()` to apply a per-segment alpha, and a parser that
 * only understood hex would read `rgb(...)` as zero and paint the bar black.
 * `channels` accepts both forms for the same reason — this is the one place
 * that has to be tolerant, so nothing downstream has to care.
 */
export class Color {
  static channels(color: string): Channels {
    if (color.startsWith('rgb')) {
      const parts = (color.match(/[\d.]+/g) ?? ['0', '0', '0']).map(Number);
      return [parts[0] || 0, parts[1] || 0, parts[2] || 0];
    }
    let hex = color.replace('#', '').trim();
    if (hex.length === 3) {
      hex = hex[0]! + hex[0]! + hex[1]! + hex[1]! + hex[2]! + hex[2]!;
    }
    const n = Number.parseInt(hex, 16);
    if (hex.length !== 6 || Number.isNaN(n)) return [255, 255, 255];
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  }

  static rgba(color: string, alpha: number): string {
    const [r, g, b] = Color.channels(color);
    return `rgba(${r},${g},${b},${alpha})`;
  }

  /** Blend two colours, returning hex so the result survives every downstream parse. */
  static mix(from: string, to: string, k: number): string {
    const a = Color.channels(from);
    const b = Color.channels(to);
    return `#${Color.#byte(Ease.lerp(a[0], b[0], k))}${Color.#byte(Ease.lerp(a[1], b[1], k))}${Color.#byte(Ease.lerp(a[2], b[2], k))}`;
  }

  static #byte(v: number): string {
    return Math.round(Ease.clamp(v, 0, 255))
      .toString(16)
      .padStart(2, '0');
  }
}
