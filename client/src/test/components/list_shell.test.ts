import { render } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";

const { default: ListShell } = await import("../../components/settings/ListShell.svelte");

function mount(props: Record<string, unknown> = {}) {
    const host = document.createElement("div");
    document.body.append(host);
    render(ListShell, {
        target: host,
        props: {
            state: "ready",
            count: 3,
            failTitle: "Couldn't load your Realms",
            failNote: "Xbox Live returned 503.",
            emptyTitle: "No Realms on this account",
            emptyNote: "Realms you own appear here.",
            children: undefined,
            ...props,
        } as never,
    });
    return {
        host,
        text: () => host.textContent ?? "",
        skeletons: () => host.querySelectorAll(".rad-skeleton").length,
        empty: () => host.querySelector(".rad-empty"),
        retry: () => host.querySelector<HTMLElement>(".rad-btn--primary"),
    };
}

describe("ListShell", () => {
    // A skeleton says something is coming and roughly what shape it is. A spinner says
    // only that you should keep waiting.
    it("shows the shape of what is coming while loading", () => {
        const view = mount({ state: "loading" });
        expect(view.skeletons()).toBeGreaterThan(0);
        expect(view.text()).not.toContain("No Realms");
    });

    // A failure needs a reason and a retry. Without them it is a dead end.
    it("gives a failure a reason and a way out", () => {
        const onretry = vi.fn();
        const view = mount({ state: "failed", onretry });
        expect(view.text()).toContain("Couldn't load your Realms");
        expect(view.text()).toContain("Xbox Live returned 503.");
        view.retry()?.click();
        expect(onretry).toHaveBeenCalledTimes(1);
    });

    // Empty is a different screen from failed: nothing is wrong, there is just nothing
    // here, and what the reader needs is how something gets here.
    it("says how something gets here when there is nothing", () => {
        const view = mount({ state: "ready", count: 0 });
        expect(view.text()).toContain("No Realms on this account");
        expect(view.text()).not.toContain("Couldn't load");
        expect(view.skeletons()).toBe(0);
    });

    // The one that matters most: a grid of tiles rendered after the server refused is an
    // offer that cannot be taken.
    it("never shows rows over a failure, even when rows are loaded", () => {
        const view = mount({ state: "failed", count: 12 });
        expect(view.empty()).not.toBeNull();
        expect(view.text()).toContain("Couldn't load your Realms");
    });

    it("retries only where a retry was given", () => {
        expect(mount({ state: "failed", onretry: undefined }).retry()).toBeNull();
    });
});
