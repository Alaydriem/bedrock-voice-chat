import type { RingSource } from "../ring/RingSource";

export interface Placement {
  /** Radians, -PI/2 straight up. Bearing relative to where the listener faces. */
  bearing: number;
  /** Metres from the listener. */
  distance: number;
  hue: string;
  /** Per-player volume, 0 to 1.5. */
  gain?: number;
}

/**
 * A player's position, turned into a voice on the ring.
 *
 * Range and falloff match the server's: past `RANGE` a voice is not audible and so
 * is not drawn, and inside it the level falls off with the square of distance,
 * which is why someone twenty metres away is much quieter rather than slightly
 * quieter.
 *
 * A player in a channel is exempt: channels are full volume at any distance, which
 * is what they are for. Pass `distance: 0` or use `inChannel`.
 */
export class PositionalSource {
  /** Metres. Matches the server's proximity range. */
  static readonly RANGE = 80;

  static falloff(distance: number, range = PositionalSource.RANGE): number {
    if (distance <= 0) return 1;
    if (distance >= range) return 0;
    const near = 1 - distance / range;
    return near * near;
  }

  /** Null when the player is out of range and has nothing to contribute. */
  static toRingSource(placement: Placement, level: number, range = PositionalSource.RANGE): RingSource | null {
    const volume = level * PositionalSource.falloff(placement.distance, range) * (placement.gain ?? 1);
    if (volume <= 0.03) return null;
    return { angle: placement.bearing, volume: Math.min(1, volume), hue: placement.hue };
  }

  static inChannel(bearing: number, hue: string, level: number, gain = 1): RingSource | null {
    const volume = level * gain;
    if (volume <= 0.03) return null;
    return { angle: bearing, volume: Math.min(1, volume), hue };
  }
}
