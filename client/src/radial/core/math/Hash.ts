/**
 * Deterministic hashing. Identity in Radial is derived, never stored: a server's
 * glyph and a player's hue both fall out of their name, so two clients showing
 * the same server agree without being told.
 */
export class Hash {
  /** FNV-1a, 32-bit unsigned. */
  static fnv1a(input: string): number {
    let h = 2166136261;
    for (let i = 0; i < input.length; i++) {
      h ^= input.charCodeAt(i);
      h = Math.imul(h, 16777619) >>> 0;
    }
    return h >>> 0;
  }

  /** Per-index scatter in [0,1). Used to desynchronise per-bar animation. */
  static scatter(index: number): number {
    const s = Math.sin(index * 12.9898 + 78.233) * 43758.5453;
    return s - Math.floor(s);
  }
}
