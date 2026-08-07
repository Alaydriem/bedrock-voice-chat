import { render } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";

const { default: Cover } = await import("../../components/shell/Cover.svelte");

function mount(props: Record<string, unknown> = {}) {
    const frame = document.createElement("div");
    frame.className = "rad-frame rad-frame--fluid";
    document.body.append(frame);

    const rendered = render(Cover, {
        target: frame,
        props: { open: false, ondismiss: () => {}, under: undefined, children: undefined, ...props },
    });

    return {
        rendered,
        frame,
        cover: () => frame.querySelector<HTMLElement>(".rad-cover"),
        handle: () => frame.querySelector<HTMLElement>(".rad-cover__handle"),
        grip: () => frame.querySelector<HTMLElement>(".rad-cover__grip"),
        under: () => frame.querySelector<HTMLElement>(".rad-under"),
        scrim: () => frame.querySelector<HTMLElement>(".rad-scrim--cover"),
    };
}

function drag(from: Element | null | undefined, startY: number, endY: number): void {
    from?.dispatchEvent(new MouseEvent("pointerdown", { clientY: startY, bubbles: true }));
    from?.dispatchEvent(new MouseEvent("pointermove", { clientY: endY, bubbles: true }));
    from?.dispatchEvent(new MouseEvent("pointerup", { clientY: endY, bubbles: true }));
}

function escape(): void {
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
}

/**
 * Watches for the cover taking the pointer.
 *
 * The browser retargets a click to whichever element holds the pointer, so a capture taken
 * on the press itself is the difference between a working button and a dead one. happy-dom
 * has no capture of its own, hence the stand-in.
 */
function watchCapture() {
    const taken = vi.fn();
    Element.prototype.setPointerCapture = taken as never;
    Element.prototype.releasePointerCapture = vi.fn() as never;
    return taken;
}

function press(on: Element | null | undefined, y = 0): void {
    on?.dispatchEvent(new MouseEvent("pointerdown", { clientY: y, bubbles: true }));
}

describe("Cover", () => {
    it("sits closed until it is opened", () => {
        const { cover, under, scrim } = mount({ open: false });
        expect(cover()?.classList.contains("is-open")).toBe(false);
        expect(under()?.classList.contains("is-covered")).toBe(false);
        expect(scrim()?.classList.contains("is-on")).toBe(false);
    });

    it("covers the screen behind it when open", () => {
        const { cover, under, scrim } = mount({ open: true });
        expect(cover()?.classList.contains("is-open")).toBe(true);
        expect(under()?.classList.contains("is-covered")).toBe(true);
        expect(scrim()?.classList.contains("is-on")).toBe(true);
    });

    // Scaling the screen behind leaves every control on it focusable, so tabbing off the
    // end of the cover lands on a dashboard button nobody can see.
    it("takes the screen behind out of the tab order while covered", () => {
        expect(mount({ open: true }).under()?.hasAttribute("inert")).toBe(true);
        expect(mount({ open: false }).under()?.hasAttribute("inert")).toBe(false);
    });

    it("dismisses on Escape", () => {
        const ondismiss = vi.fn();
        mount({ open: true, ondismiss });
        escape();
        expect(ondismiss).toHaveBeenCalledTimes(1);
    });

    it("ignores Escape when it is not open", () => {
        const ondismiss = vi.fn();
        mount({ open: false, ondismiss });
        escape();
        expect(ondismiss).not.toHaveBeenCalled();
    });

    // A menu or a modal on top owns Escape first. The cover is the surface they were
    // opened from, so closing it out from under them dismisses two things with one press.
    it("lets a modal on top take Escape first", () => {
        const ondismiss = vi.fn();
        const { frame } = mount({ open: true, ondismiss });
        const modal = document.createElement("div");
        modal.className = "rad-modal is-open";
        frame.append(modal);
        escape();
        expect(ondismiss).not.toHaveBeenCalled();
    });

    it("lets an open menu take Escape first", () => {
        const ondismiss = vi.fn();
        const { frame } = mount({ open: true, ondismiss });
        const menu = document.createElement("div");
        menu.className = "rad-menu is-open";
        frame.append(menu);
        escape();
        expect(ondismiss).not.toHaveBeenCalled();
    });

    it("dismisses when the scrim is clicked", () => {
        const ondismiss = vi.fn();
        const { scrim } = mount({ open: true, ondismiss });
        scrim()?.click();
        expect(ondismiss).toHaveBeenCalledTimes(1);
    });

    // The scrim is inert while closed, and a stray click on the dashboard behind it must
    // not fire a dismiss for a cover that is not there.
    it("does not dismiss from the scrim while closed", () => {
        const ondismiss = vi.fn();
        const { scrim } = mount({ open: false, ondismiss });
        scrim()?.click();
        expect(ondismiss).not.toHaveBeenCalled();
    });

    // The cover stops short of the top edge so it reads as a sheet over the dashboard. The
    // handle is what says that edge can be grabbed.
    it("shows a grab affordance", () => {
        const view = mount({ open: true });
        expect(view.handle()).not.toBeNull();
        // Decorative. A handle in the tab order would be an unlabelled control sitting
        // ahead of the back button and Escape, which are the accessible ways out.
        expect(view.grip()?.getAttribute("aria-hidden")).toBe("true");
    });

    // The visible bar is 4px tall, which no finger can hit. The grip around it is the
    // target, and it has to be the thing the gesture starts on.
    it("drags from the grip, not only from the bar", () => {
        const ondismiss = vi.fn();
        const view = mount({ open: true, ondismiss });
        drag(view.grip(), 0, 200);
        expect(ondismiss).toHaveBeenCalledTimes(1);
    });

    it("drags from the bar inside the grip too", () => {
        const ondismiss = vi.fn();
        const view = mount({ open: true, ondismiss });
        drag(view.handle(), 0, 200);
        expect(ondismiss).toHaveBeenCalledTimes(1);
    });

    it("dismisses on a drag past the threshold", () => {
        const ondismiss = vi.fn();
        const { cover } = mount({ open: true, ondismiss });
        drag(cover(), 0, 200);
        expect(ondismiss).toHaveBeenCalledTimes(1);
    });

    // Released short of the threshold the cover springs back. Dismissing on any downward
    // movement would make the screen impossible to hold still.
    it("springs back from a drag that stops short", () => {
        const ondismiss = vi.fn();
        const { cover } = mount({ open: true, ondismiss });
        drag(cover(), 0, 40);
        expect(ondismiss).not.toHaveBeenCalled();
        expect(cover()?.getAttribute("style") ?? "").not.toContain("translateY");
    });

    // The rule that makes "drag from anywhere" survivable: partway down a settings pane, a
    // downward drag means scroll up.
    it("yields the gesture to content that has somewhere to scroll", () => {
        const ondismiss = vi.fn();
        const { cover } = mount({ open: true, ondismiss });
        const body = document.createElement("div");
        body.className = "rad-settings-body";
        Object.defineProperty(body, "scrollTop", { value: 320, writable: true });
        cover()?.append(body);

        drag(body, 0, 200);
        expect(ondismiss).not.toHaveBeenCalled();
    });

    it("takes the gesture once that content is back at its top", () => {
        const ondismiss = vi.fn();
        const { cover } = mount({ open: true, ondismiss });
        const body = document.createElement("div");
        body.className = "rad-settings-body";
        Object.defineProperty(body, "scrollTop", { value: 0, writable: true });
        cover()?.append(body);

        drag(body, 0, 200);
        expect(ondismiss).toHaveBeenCalledTimes(1);
    });

    // A slider is dragged for its own reasons, and a modal owns its own gestures.
    it("ignores a drag that began on a control", () => {
        const ondismiss = vi.fn();
        const { cover } = mount({ open: true, ondismiss });
        const slider = document.createElement("input");
        slider.className = "rad-range";
        slider.type = "range";
        cover()?.append(slider);

        drag(slider, 0, 200);
        expect(ondismiss).not.toHaveBeenCalled();
    });

    // Every control on the settings screen is a button inside this cover. A press that takes
    // the pointer sends the click to the cover instead of to the button, so the pane rendered
    // and read correctly and nothing on it could be operated — including the close button.
    it("leaves the pointer with a button that was pressed", () => {
        const taken = watchCapture();
        const { cover } = mount({ open: true });
        const button = document.createElement("button");
        cover()?.append(button);

        press(button);
        expect(taken).not.toHaveBeenCalled();
    });

    it("ignores a drag that began on a button", () => {
        const ondismiss = vi.fn();
        const { cover } = mount({ open: true, ondismiss });
        const button = document.createElement("button");
        cover()?.append(button);

        drag(button, 0, 200);
        expect(ondismiss).not.toHaveBeenCalled();
    });

    // The guard above names the controls it knows about. This is what covers the ones it does
    // not — a label, a role=button, anything a pane invents later: until the gesture has
    // travelled, the press still belongs to whatever was pressed.
    it("does not take the pointer until the press has become a drag", () => {
        const taken = watchCapture();
        const { cover } = mount({ open: true });

        press(cover());
        expect(taken).not.toHaveBeenCalled();

        cover()?.dispatchEvent(new MouseEvent("pointermove", { clientY: 60, bubbles: true }));
        expect(taken).toHaveBeenCalledTimes(1);
    });

    it("ignores a drag while it is closed", () => {
        const ondismiss = vi.fn();
        const { cover } = mount({ open: false, ondismiss });
        drag(cover(), 0, 200);
        expect(ondismiss).not.toHaveBeenCalled();
    });
});
