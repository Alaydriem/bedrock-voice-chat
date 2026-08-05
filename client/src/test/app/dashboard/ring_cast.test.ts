import { describe, expect, it } from "vitest";
import { RingCast } from "../../../js/app/dashboard/RingCast";
import type { NearbyPlayer } from "../../../js/app/dashboard/NearbyPlayer";

function player(distance: number, bearing = 0): NearbyPlayer {
    return {
        name: `minecraft:P${distance}`,
        gamertag: `P${distance}`,
        game: "minecraft",
        hue: "#8239d8",
        presence: "voice",
        distance,
        bearing,
        elevation: 0,
        inEarshot: false,
    } as NearbyPlayer;
}

const SCOPE = 240;

describe("RingCast", () => {
    /**
     * The register the loader shows while it waits.
     *
     * `empty` is the at-rest ring, and at rest was the wrong claim: a proximity client with
     * nobody nearby is not idle, it is looking. Showing the same dull ring for "listening" and
     * for "the link is dead" left the two indistinguishable.
     */
    it("scans rather than resting when nobody is nearby", () => {
        const state = RingCast.of([], SCOPE);

        expect(state.mode).toBe("live");
        expect(state.sources).toHaveLength(0);
    });

    // `lock` is documented as one source acquiring, which is what a single approach is.
    it("locks on to a single approach", () => {
        expect(RingCast.of([player(90)], SCOPE).mode).toBe("lock");
    });

    it("goes live once there is more than one", () => {
        expect(RingCast.of([player(90), player(140)], SCOPE).mode).toBe("live");
    });

    it("places at most five marks, so the ring stays a reading", () => {
        const crowd = [40, 60, 80, 100, 120, 140, 160, 180].map((d) => player(d));

        expect(RingCast.of(crowd, SCOPE).sources).toHaveLength(RingCast.MAX_MARKS);
    });

    /**
     * The one case that really is nothing happening.
     *
     * Marks would assert positions this client can no longer be told about, and a scanning ring
     * would claim the system is looking when it has nothing to look through.
     */
    it("rests, and draws nobody, when the link is down", () => {
        const state = RingCast.of([player(90), player(140)], SCOPE, false);

        expect(state.mode).toBe("empty");
        expect(state.sources).toHaveLength(0);
    });
});
