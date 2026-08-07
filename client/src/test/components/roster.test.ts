import { render } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import { ConstantLevelSource } from "$radial/core/sources/LevelSource";

const { default: Roster } = await import("../../components/dashboard/Roster.svelte");

const silent = new ConstantLevelSource(0);

function mount(props: Record<string, unknown> = {}) {
    const host = document.createElement("div");
    document.body.append(host);
    render(Roster, {
        target: host,
        props: {
            title: "Raid party",
            players: [],
            sourceFor: () => silent,
            gainFor: () => 1,
            mutedFor: () => false,
            onmute: () => {},
            ongain: () => {},
            opened: null,
            onopen: () => {},
            ...props,
        } as never,
    });
    return {
        host,
        text: () => host.textContent ?? "",
        section: () => host.querySelector(".rad-roster__section"),
        cards: () => host.querySelectorAll(".rad-card-grid").length,
    };
}

describe("Roster", () => {
    /**
     * A channel you are alone in still needs its rule and its way out.
     *
     * Hiding the section because the member list is empty left somebody joined with no
     * visible exit and no evidence on the dashboard that they were in a group at all — and
     * being first into a group you just made is the ordinary way to arrive in one.
     */
    it("keeps a section that has an empty line to show", () => {
        const view = mount({ empty: "Nobody else is in here yet." });

        expect(view.section()).not.toBeNull();
        expect(view.text()).toContain("Raid party");
        expect(view.text()).toContain("Nobody else is in here yet.");
        // No grid at all rather than an empty one, so nothing contributes spacing for cards
        // that are not there.
        expect(view.cards()).toBe(0);
    });

    // Earshot passes no empty line: an empty proximity list is the ring's state, and a
    // section announcing it would duplicate what the ring already says.
    it("stays hidden when it has nobody and nothing to say", () => {
        const view = mount();

        expect(view.section()).toBeNull();
        expect(view.text()).not.toContain("Raid party");
    });

    it("draws its members once there are any", () => {
        const view = mount({
            empty: "Nobody else is in here yet.",
            players: [
                {
                    name: "minecraft:Petra",
                    gamertag: "Petra",
                    game: "minecraft",
                    hue: "#fff",
                    presence: "voice",
                    distance: 0,
                    bearing: 0,
                    elevation: 0,
                    inEarshot: true,
                },
            ],
        });

        expect(view.text()).toContain("Petra");
        expect(view.text()).not.toContain("Nobody else is in here yet.");
    });
});
