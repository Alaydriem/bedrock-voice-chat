/**
 * A voice, placed on the ring.
 *
 * `angle` is radians, with -PI/2 straight up. `volume` is 0 to 1 and sets how far
 * the bars near that angle push outward. `hue` tints them, which is the whole
 * mechanism by which the ring says *who* without carrying a name: a player's hue
 * is theirs everywhere in the product.
 */
export interface RingSource {
  angle: number;
  volume: number;
  hue: string;
}
