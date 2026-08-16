import { render } from "@testing-library/svelte";
import { flushSync } from "svelte";
import { describe, expect, it, vi } from "vitest";
import type { GroupRowView } from "../../js/app/dashboard/GroupRowView";

const { default: GroupRow } = await import("../../components/dashboard/GroupRow.svelte");

function view(over: Partial<GroupRowView> = {}): GroupRowView {
    return {
        id: "g1",
        name: "Quiet Meadow",
        members: [],
        joined: true,
        owned: false,
        activeAt: null,
        stirring: false,
        ...over,
    };
}

function mount(group: GroupRowView = view()) {
    const host = document.createElement("div");
    document.body.append(host);
    const onjoin = vi.fn();
    const onopen = vi.fn();
    render(GroupRow as never, {
        target: host,
        props: { group, now: Date.now(), onjoin, onopen },
    } as never);
    const row = host.querySelector<HTMLElement>(".rad-group-row") as HTMLElement;
    const track = host.querySelector<HTMLElement>(".rad-swipe__track") as HTMLElement;
    return { host, row, track, onjoin, onopen };
}

function pointer(el: Element, type: string, clientX = 0): void {
    el.dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 1, clientX }));
}

describe("GroupRow swipe", () => {
    it("moves the track while a swipe is in progress", () => {
        const { row, track } = mount();
        pointer(row, "pointerdown", 200);
        pointer(row, "pointermove", 100);
        flushSync();
        expect(track.style.transform).not.toBe("translateX(0px)");
        expect(track.classList.contains("is-dragging")).toBe(true);
    });

    // On touch, pointerdown implicitly captures the pointer to the element under the
    // finger — a span inside the row. Taking the capture for the row transfers it, and the
    // child announces the loss with a lostpointercapture that bubbles here. That is the
    // start of every touch swipe, not the end of one.
    it("keeps swiping when a child loses its implicit capture", () => {
        const { row, track } = mount();
        const inner = row.querySelector(".rad-group-row__name") as Element;

        inner.dispatchEvent(
            new PointerEvent("pointerdown", { bubbles: true, pointerId: 1, clientX: 200 }),
        );
        // The transfer: the pressed child gives its implicit capture up to the row.
        inner.dispatchEvent(new PointerEvent("lostpointercapture", { bubbles: true }));

        inner.dispatchEvent(
            new PointerEvent("pointermove", { bubbles: true, pointerId: 1, clientX: 100 }),
        );
        flushSync();
        expect(track.style.transform).not.toBe("translateX(0px)");
        expect(track.classList.contains("is-dragging")).toBe(true);
    });

    // The browser can take the capture back without a pointerup — the webview backgrounded
    // mid-swipe is the reproducible case. The row must return to where it rested, and the
    // interruption must not read as the user opening or closing the tray.
    it("springs back when the pointer capture is lost mid-swipe", () => {
        const { row, track, onopen } = mount();
        pointer(row, "pointerdown", 200);
        pointer(row, "pointermove", 100);
        flushSync();

        pointer(row, "lostpointercapture");
        flushSync();
        expect(track.style.transform).toBe("translateX(0px)");
        expect(track.classList.contains("is-dragging")).toBe(false);
        expect(onopen).not.toHaveBeenCalled();
    });

    it("does not resume an interrupted swipe from later pointer events", () => {
        const { row, track, onopen } = mount();
        pointer(row, "pointerdown", 200);
        pointer(row, "pointermove", 100);
        pointer(row, "lostpointercapture");

        // Stale continuations of the dead gesture. Neither may move the row nor latch
        // the tray.
        pointer(row, "pointermove", 20);
        flushSync();
        expect(track.style.transform).toBe("translateX(0px)");
        pointer(row, "pointerup", 20);
        expect(onopen).not.toHaveBeenCalled();
    });
});
