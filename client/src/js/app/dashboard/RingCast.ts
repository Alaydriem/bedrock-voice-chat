import type { RingMode } from '$radial/bindings/RingBinding';
import { PositionalSource } from '$radial/core/sources/PositionalSource';
import type { RingSource } from '$radial/core/ring/RingSource';
import type { NearbyPlayer } from './NearbyPlayer';

export interface RingState {
    mode: RingMode;
    sources: readonly RingSource[];
}

/**
 * What the ring is doing while nobody is in earshot yet.
 *
 * Three states, and the middle one is the reason the feed reaches past voice range at all:
 * scanning says the system is listening; with somebody approaching it reaches out to them; once
 * anyone is close enough to hear, the roster takes over and the ring gets out of the way.
 *
 * `lock` for one and `live` for several is not a workaround — `lock` is documented as one
 * source acquiring, which is exactly what a single approach is.
 */
export class RingCast {
    /** Marks placed at once. More than this and the ring is a crowd rather than a reading. */
    static readonly MAX_MARKS = 5;

    /**
     * @param connected False when the link is down, which outranks everything below it.
     */
    static of(
        approaching: readonly NearbyPlayer[],
        scope: number,
        connected = true,
    ): RingState {
        // A ring at rest is the only honest reading for a link that is down, and the only place
        // `empty` still belongs. Drawing marks would assert positions this client can no longer
        // be told about.
        if (!connected) {
            return { mode: 'empty', sources: [] };
        }

        if (approaching.length === 0) {
            // `live` with no sources, not `empty`.
            //
            // A proximity client with nobody nearby is not at rest, it is looking — the same
            // activity the loader draws while it waits, and the same register. `empty` was
            // saying the system had stopped.
            return { mode: 'live', sources: [] };
        }

        const sources = approaching
            .slice(0, RingCast.MAX_MARKS)
            .map((player) =>
                // Volume from distance rather than from their voice: they are out of earshot,
                // so there is no voice to draw. What the mark carries is how close they are.
                PositionalSource.toRingSource(
                    { bearing: player.bearing, distance: player.distance, hue: player.hue },
                    1,
                    scope,
                ),
            )
            .filter((source): source is RingSource => source !== null);

        return { mode: approaching.length === 1 ? 'lock' : 'live', sources };
    }
}
