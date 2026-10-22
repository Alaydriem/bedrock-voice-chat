import { describe, expect, it } from "vitest";
import {
    AdmissionStackView,
    type AdmissionCardState,
} from "../../../js/app/dashboard/AdmissionStackView";

function card(joinerSocket: number, over: Partial<AdmissionCardState> = {}): AdmissionCardState {
    return {
        joinerSocket,
        name: `minecraft:P${joinerSocket}`,
        phase: "request",
        checkId: null,
        code: null,
        otherMember: null,
        pending: false,
        receivedAt: 0,
        ...over,
    };
}

describe("AdmissionStackView", () => {
    // Several codes on screen at once let a member compare a spoken code against the wrong
    // card. Only one card is ever open on its code.
    it("expanding one card collapses the other", () => {
        const view = new AdmissionStackView();
        const cards = [
            card(1, { phase: "code", checkId: "a", code: "1111 1111" }),
            card(2, { phase: "code", checkId: "b", code: "2222 2222" }),
        ];

        view.expand(1);
        view.expand(2);
        const laid = view.layout(cards, 0);

        expect(laid.cards.map((c) => c.expanded)).toEqual([false, true]);
    });

    it("the first code to arrive opens on its own; later ones wait collapsed", () => {
        const view = new AdmissionStackView();

        view.codeArrived(1);
        view.codeArrived(2);

        expect(view.expandedJoiner()).toBe(1);
    });

    it("counts the requests waiting beyond the shown cards", () => {
        const view = new AdmissionStackView();

        expect(view.layout([card(1)], 1).more).toBe(1);
        expect(view.layout([card(1)], 0).more).toBe(0);
    });

    it("allows no fourth check while three are open", () => {
        const view = new AdmissionStackView();
        const cards = [
            card(1, { phase: "code", checkId: "a" }),
            card(2, { pending: true }),
            card(3, { phase: "code", checkId: "c" }),
            card(4),
        ];

        const laid = view.layout(cards, 0);

        expect(laid.cards[3].canCheck).toBe(false);
    });
});
