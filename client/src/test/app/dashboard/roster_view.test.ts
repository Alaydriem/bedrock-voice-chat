import { describe, expect, it } from "vitest";
import { RosterView } from "../../../js/app/dashboard/RosterView";
import type { NearbyPlayer } from "../../../js/app/dashboard/NearbyPlayer";

function player(distance: number): NearbyPlayer {
    return {
        name: `minecraft:P${distance}`,
        gamertag: `P${distance}`,
        game: "minecraft",
        hue: "#fff",
        presence: "voice",
        distance,
        bearing: 0,
        elevation: 0,
        inEarshot: true,
    };
}

function crowd(size: number): NearbyPlayer[] {
    return Array.from({ length: size }, (_, i) => player(i + 1));
}

describe("RosterView.split", () => {
    it("gives everybody a card below the threshold", () => {
        const split = RosterView.split(crowd(RosterView.DENSE_AT - 1));

        expect(split.cards).toHaveLength(RosterView.DENSE_AT - 1);
        expect(split.chips).toHaveLength(0);
    });

    it("keeps six cards and turns the rest into avatars once the room is dense", () => {
        const split = RosterView.split(crowd(40));

        expect(split.cards).toHaveLength(RosterView.CARD_LIMIT);
        expect(split.chips).toHaveLength(40 - RosterView.CARD_LIMIT);
    });

    it("nobody appears twice", () => {
        const split = RosterView.split(crowd(40));
        const names = new Set([...split.cards, ...split.chips].map((p) => p.name));

        expect(names.size).toBe(40);
    });

    // The cards are the nearest six, not the six who are talking: promoting on speech changes
    // the set every couple of seconds and reflows every avatar below it.
    it("cards the nearest, so the set does not churn as people speak", () => {
        const split = RosterView.split(crowd(40));

        expect(split.cards.map((p) => p.distance)).toEqual([1, 2, 3, 4, 5, 6]);
    });
});

describe("RosterView.headline", () => {
    it("counts earshot when anybody is close enough to hear", () => {
        expect(RosterView.headline(3, 7)).toBe("3 IN EARSHOT");
    });

    // The state the ring owns: nobody audible, but somebody on the way.
    it("counts the approach when nobody is in earshot yet", () => {
        expect(RosterView.headline(0, 2)).toBe("2 NEARBY");
    });

    it("says so plainly when the field is empty", () => {
        expect(RosterView.headline(0, 0)).toBe("NOBODY IN EARSHOT");
    });
});
