/**
 * A measured RMS turned into a level a meter can show.
 *
 * RMS is linear in pressure and hearing is not, so feeding it straight to a meter spends
 * almost the entire range on levels nobody produces. Speech into a desk microphone sits
 * around 0.02 to 0.08 RMS — under a tenth of full scale — so a linear meter reads flat
 * until someone is against the capsule, and by then they are clipping.
 *
 * The mapping is decibels across a window chosen to be generous rather than accurate.
 * This is not a mixing tool: nobody is setting a broadcast level here, they are answering
 * "does BVC hear me", and the answer wants to be unmistakable.
 */
export class LevelScale {
  /** Below this reads as silence. Roughly a quiet room once the gate has had its say. */
  static readonly FLOOR_DB = -55;

  /** At or above this the meter is full. Well under clipping, deliberately. */
  static readonly CEILING_DB = -18;

  /** Post-gate RMS in 0..1, to a display level in 0..1. */
  static fromRms(rms: number): number {
    // Guards NaN as well as zero and negatives: log10(0) is -Infinity, and a gated frame
    // arrives as exactly 0.
    if (!(rms > 0)) return 0;

    const db = 20 * Math.log10(rms);
    const span = LevelScale.CEILING_DB - LevelScale.FLOOR_DB;
    return LevelScale.clamp01((db - LevelScale.FLOOR_DB) / span);
  }

  private static clamp01(value: number): number {
    return value < 0 ? 0 : value > 1 ? 1 : value;
  }
}
