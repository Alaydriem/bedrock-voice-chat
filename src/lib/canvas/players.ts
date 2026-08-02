/**
 * The cast used by every visual on the site.
 *
 * Shared so the same three names and colours appear in the ring, the channel
 * panel and the multitrack view — a viewer scrolling the page is watching one
 * session from different angles, not three unrelated demos. Colours are
 * spectrum stops, indices 7, 12 and 18 of the mark.
 */
export interface Player {
  readonly name: string;
  readonly hue: string;
  /** Starting bearing in radians. */
  readonly bearing: number;
  /** Starting distance, 0 = on top of you, 1 = edge of range. */
  readonly distance: number;
  /** Orbit speed, radians per millisecond. */
  readonly speed: number;
  /** Drift speed for the distance wobble. */
  readonly drift: number;
  /** Phase offset so they do not move in lockstep. */
  readonly phase: number;
}

export const PLAYERS: readonly Player[] = [
  { name: 'ALAYDRIEM', hue: '#21d8d8', bearing: 0.55, distance: 0.3, speed: 0.0004, drift: 0.00009, phase: 0 },
  { name: 'PETRA', hue: '#aee236', bearing: 2.55, distance: 0.54, speed: -0.00029, drift: 0.00006, phase: 1.7 },
  { name: 'JUNO', hue: '#f67414', bearing: 4.45, distance: 0.82, speed: 0.0002, drift: 0.00004, phase: 3.1 },
];

export interface PlayerState {
  bearing: number;
  distance: number;
  /** 0 when out of range, 1 when right next to you. */
  volume: number;
}

/** Where a player is, and how loud they are, at time t. */
export function playerAt(p: Player, t: number): PlayerState {
  const bearing = p.bearing + t * p.speed;
  const distance = Math.max(0.05, Math.min(1.14, p.distance + Math.sin(t * p.drift + p.phase) * 0.13));
  return { bearing, distance, volume: Math.max(0, 1 - distance / 0.9) };
}
