import { render } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import type { ChatLine } from "../../js/app/chat/ChatLine";

const { default: ChatMessageRow } = await import(
    "../../components/dashboard/ChatMessageRow.svelte"
);

function line(over: Partial<ChatLine> = {}): ChatLine {
    return {
        author: "Petra",
        text: "anyone got spare iron",
        system: false,
        fromApp: false,
        mention: false,
        timestamp: "14:03",
        ...over,
    };
}

function mount(l: ChatLine) {
    const { container } = render(ChatMessageRow, { props: { line: l, hue: "#8239d8" } });
    return container;
}

/**
 * The kit defines exactly one event style — `rad-msg--system` — and the radial reference uses
 * it for joins, leaves and deaths alike. These assert the rendered shape matches that, because
 * "it should look right" is not something inspection can keep true.
 */
describe("ChatMessageRow events", () => {
    it("renders a join as a system line", () => {
        const el = mount(line({ system: true, author: null, text: "Petra joined the game" }));

        expect(el.querySelector(".rad-msg--system")).not.toBeNull();
        expect(el.querySelector(".rad-msg__text")?.textContent).toBe("Petra joined the game");
    });

    it("renders a death as a system line", () => {
        const el = mount(
            line({ system: true, author: null, text: "Moth was slain by Enderman" }),
        );

        expect(el.querySelector(".rad-msg--system")).not.toBeNull();
    });

    it("renders a server say as a system line", () => {
        const el = mount(line({ system: true, author: null, text: "[Server] hello" }));

        expect(el.querySelector(".rad-msg--system")).not.toBeNull();
    });

    // No hue and no name: the kit dims the text and blanks the avatar's background off this
    // class, and an author would make the server look like a player.
    it("gives an event the neutral dot rather than an author or a hue", () => {
        const el = mount(line({ system: true, author: null, text: "Wren left the game" }));

        expect(el.querySelector(".rad-msg__avatar")?.textContent?.trim()).toBe("·");
        expect(el.querySelector(".rad-msg__author")).toBeNull();
        expect(el.querySelector<HTMLElement>(".rad-msg__avatar")?.style.background).toBe("");
    });

    it("keeps a player line out of the system style", () => {
        const el = mount(line());

        expect(el.querySelector(".rad-msg--system")).toBeNull();
        expect(el.querySelector(".rad-msg__author")?.textContent).toBe("Petra");
    });

    it("marks an app-sent line so it is distinguishable from one typed in game", () => {
        const el = mount(line({ fromApp: true }));

        expect(el.querySelector(".rad-msg__app")).not.toBeNull();
    });

    it("marks a mention", () => {
        const el = mount(line({ mention: true }));

        expect(el.querySelector(".rad-msg--mention")).not.toBeNull();
    });
});
