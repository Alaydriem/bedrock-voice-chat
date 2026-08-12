import { render } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import "../tauri";
import type { SelfSnapshot, VoiceMode } from "../../radial/core/controllers/SelfState";

const { default: SelfPill } = await import("../../radial/components/SelfPill.svelte");

function snapshot(mode: VoiceMode, over: Partial<SelfSnapshot> = {}): SelfSnapshot {
    return {
        muted: false,
        deafened: false,
        recording: false,
        mode,
        holding: false,
        transmitting: mode === "activated",
        recordAllowed: true,
        captureAvailable: true,
        ...over,
    };
}

function mount(state: SelfSnapshot) {
    const host = document.createElement("div");
    document.body.append(host);
    const onmute = vi.fn();
    const onhold = vi.fn();
    const onrecord = vi.fn();
    render(SelfPill as never, {
        target: host,
        props: { name: "Alaydriem", state, onmute, onhold, onrecord },
    } as never);
    // By class, not by label: the label is one of the things under test, and finding the
    // control by the string it is asserted to have makes the assertion circular.
    const mic = host.querySelector<HTMLElement>(
        ".rad-self__btn:not(.rad-self__btn--deafen):not(.rad-self__btn--record)",
    ) as HTMLElement;
    const record = host.querySelector<HTMLButtonElement>(
        ".rad-self__btn--record",
    ) as HTMLButtonElement;
    return { host, mic, record, onmute, onhold, onrecord };
}

function pointer(el: Element, type: string): void {
    el.dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 1 }));
}

describe("SelfPill mic button in open mic", () => {
    it("is a toggle", () => {
        const view = mount(snapshot("activated"));
        expect(view.mic.getAttribute("aria-label")).toBe("Mute");

        view.mic.click();
        expect(view.onmute).toHaveBeenCalledTimes(1);
        expect(view.onhold).not.toHaveBeenCalled();
    });
});

describe("SelfPill mic button in push-to-talk", () => {
    // Not holding already is mute, so a toggle beside it would be a second word for the
    // same state — and pressing it would turn push-to-talk into an open mic.
    it("is a hold, not a toggle", () => {
        const view = mount(snapshot("ptt"));
        expect(view.mic.getAttribute("aria-label")).toBe("Muted. Hold to talk");

        view.mic.click();
        expect(view.onmute).not.toHaveBeenCalled();

        pointer(view.mic, "pointerdown");
        expect(view.onhold).toHaveBeenCalledWith(true);
        pointer(view.mic, "pointerup");
        expect(view.onhold).toHaveBeenLastCalledWith(false);
    });

    /**
     * A cancelled gesture never sends its release.
     *
     * On a phone the browser can read a press on a button as the start of a scroll and
     * answer with `pointercancel` instead of `pointerup`. Unhandled, that is a microphone
     * left open after the finger lifted.
     */
    it("releases the hold when the gesture is cancelled", () => {
        const view = mount(snapshot("ptt"));

        pointer(view.mic, "pointerdown");
        pointer(view.mic, "pointercancel");
        expect(view.onhold).toHaveBeenLastCalledWith(false);
    });

    it("releases the hold when the finger leaves the button", () => {
        const view = mount(snapshot("ptt"));

        pointer(view.mic, "pointerdown");
        pointer(view.mic, "pointerleave");
        expect(view.onhold).toHaveBeenLastCalledWith(false);
    });

    /**
     * Muted is drawn here too.
     *
     * Hiding it — on the grounds that muted is the resting state of push-to-talk — left the
     * button looking like an open microphone whenever nobody was holding it, which is the
     * opposite of what the mode means. The label carries the difference between this mute
     * and one you have to undo.
     */
    it("shows the muted glyph at rest", () => {
        const view = mount(snapshot("ptt", { muted: true }));
        expect(view.mic.querySelector('[data-rad-icon="micoff"]')).not.toBeNull();
        expect(view.mic.getAttribute("aria-label")).toBe("Muted. Hold to talk");
    });

    it("shows an open mic while it is held", () => {
        const view = mount(snapshot("ptt", { muted: false, holding: true, transmitting: true }));
        expect(view.mic.querySelector('[data-rad-icon="mic"]')).not.toBeNull();
        expect(view.mic.getAttribute("aria-label")).toBe("Talking, release to stop");
    });

    // The backend closes the mic a beat after the release, so `muted` lags the gesture. The
    // glyph follows the gesture: a struck-through mic mid-sentence reads as a cut-off.
    it("keeps the mic open through the release tail", () => {
        const view = mount(snapshot("ptt", { muted: true, holding: true, transmitting: true }));
        expect(view.mic.querySelector('[data-rad-icon="micoff"]')).toBeNull();
    });

    it("marks itself while it is held", () => {
        const held = mount(snapshot("ptt", { holding: true, transmitting: true }));
        expect(held.mic.classList.contains("is-holding")).toBe(true);

        const idle = mount(snapshot("ptt"));
        expect(idle.mic.classList.contains("is-holding")).toBe(false);
    });
});

describe("SelfPill record button", () => {
    it("disables the record button and says why where the server disallows recording", () => {
        const { record } = mount(snapshot("activated", { recordAllowed: false }));

        expect(record).not.toBeNull();
        expect(record.disabled).toBe(true);
        expect(record.title).toContain("Recording is off on this server");
    });

    it("leaves the record button live where the server allows recording", () => {
        const { record } = mount(snapshot("activated", { recordAllowed: true }));

        expect(record.disabled).toBe(false);
    });

    // The button stays in the layout rather than disappearing: a creator should be able
    // to see that the feature exists and that this server is the reason it is not here.
    it("keeps the record button present when recording is disallowed", () => {
        const { host } = mount(snapshot("activated", { recordAllowed: false }));

        expect(host.querySelector(".rad-self__btn--record")).not.toBeNull();
    });

    it("does not fire the record handler from a disabled button", () => {
        const { record, onrecord } = mount(snapshot("activated", { recordAllowed: false }));

        record.click();

        expect(onrecord).not.toHaveBeenCalled();
    });
});

describe("SelfPill when the microphone cannot be opened", () => {
    // A muted glyph would say the user did this. They did not, and pressing the button they
    // would reach for cannot fix it.
    it("draws the mic as unavailable and refuses the toggle", () => {
        const view = mount(snapshot("activated", { captureAvailable: false }));

        expect(view.mic.getAttribute("aria-label")).toBe("Microphone unavailable");
        expect(view.mic.hasAttribute("disabled")).toBe(true);

        view.mic.click();
        expect(view.onmute).not.toHaveBeenCalled();
    });

    it("is unaffected while capture is available", () => {
        const view = mount(snapshot("activated"));

        expect(view.mic.getAttribute("aria-label")).toBe("Mute");
        expect(view.mic.hasAttribute("disabled")).toBe(false);
    });
});
