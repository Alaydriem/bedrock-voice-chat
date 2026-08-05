import { render, waitFor } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import "../tauri";

const { default: Harness } = await import("./CoverDashboardHarness.svelte");

/**
 * The dashboard as the app actually assembles it.
 *
 * Settings is a cover over the dashboard, so the whole dashboard is now a snippet inside
 * `.rad-under` rather than a direct child of the frame. `DashboardScreen` reaches upward
 * for `.rad-frame` to put its state classes on, and `.rad-status` is `opacity: 0` until
 * one of them arrives — so the extra layer is exactly the kind of thing that would leave
 * the panel mounted, correct, and invisible.
 */
function mount(coverOpen = false) {
    const host = document.createElement("div");
    document.body.append(host);
    const rendered = render(Harness as never, { target: host, props: { coverOpen } } as never);
    return {
        host,
        rendered,
        frame: host.querySelector(".rad-frame") as HTMLElement,
        under: host.querySelector(".rad-under") as HTMLElement,
    };
}

describe("the dashboard under the settings cover", () => {
    it("keeps the frame within reach of the dashboard", () => {
        const view = mount();
        expect(view.under.querySelector(".rad-shell")).not.toBeNull();
        expect((view.host.querySelector(".rad-status") as HTMLElement).closest(".rad-frame")).toBe(
            view.frame,
        );
    });

    it("opens the status panel from the desktop button", async () => {
        const view = mount();
        expect(view.frame.classList.contains("is-status")).toBe(false);

        view.host.querySelector<HTMLElement>('[aria-label="Show status"]')?.click();
        await waitFor(() => expect(view.frame.classList.contains("is-status")).toBe(true));
    });

    // The phone has no status button in the header; the sheet is its only route in.
    it("opens the status panel from the phone sheet", async () => {
        const view = mount();
        view.host.querySelector<HTMLElement>('[aria-label="Servers and settings"]')?.click();

        const row = await waitFor(() => {
            const found = [...view.host.querySelectorAll<HTMLElement>(".rad-list-row")].find((e) =>
                e.textContent?.includes("Connection status"),
            );
            expect(found).not.toBeUndefined();
            return found as HTMLElement;
        });
        row.click();
        await waitFor(() => expect(view.frame.classList.contains("is-status")).toBe(true));
    });

    /**
     * `inert` is what keeps the covered dashboard out of the tab order, and it is inert for
     * any value at all — `inert="false"` included. Left behind on the way back from
     * settings it would take every control on the dashboard with it.
     */
    it("hands the dashboard back after a settings visit", async () => {
        const view = mount(false);

        await view.rendered.rerender({ coverOpen: true } as never);
        expect(view.under.hasAttribute("inert")).toBe(true);

        await view.rendered.rerender({ coverOpen: false } as never);
        expect(view.under.hasAttribute("inert")).toBe(false);

        view.host.querySelector<HTMLElement>('[aria-label="Show status"]')?.click();
        await waitFor(() => expect(view.frame.classList.contains("is-status")).toBe(true));
    });
});
