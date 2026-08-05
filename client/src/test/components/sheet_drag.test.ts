import { describe, expect, it } from "vitest";
import { Sheet } from "../../radial/core/controllers/Sheet";
import { CoverDrag } from "../../radial/core/controllers/CoverDrag";

function frame() {
    const el = document.createElement("div");
    el.className = "rad-frame";
    el.innerHTML = `
        <div class="rad-scrim" data-rad-sheet-scrim></div>
        <button data-rad-sheet-open="groups">open</button>
        <div class="rad-sheet rad-sheet--full" data-rad-sheet="groups">
            <span class="rad-sheet__handle"></span>
            <div class="rad-sheet__body"><button class="rad-list-row">A group</button></div>
        </div>`;
    document.body.append(el);
    return el;
}

function drag(target: Element, from: number, to: number): void {
    const opts = { bubbles: true, pointerId: 1 };
    target.dispatchEvent(new PointerEvent("pointerdown", { ...opts, clientY: from }));
    target.dispatchEvent(new PointerEvent("pointermove", { ...opts, clientY: to }));
    target.dispatchEvent(new PointerEvent("pointerup", { ...opts, clientY: to }));
}

describe("Sheet drag to dismiss", () => {
    // The handle was a bar that looked draggable and was not. The one gesture a bottom
    // sheet advertises did nothing.
    it("closes on a drag past the dismiss distance", () => {
        const el = frame();
        const sheet = new Sheet(el);
        sheet.open("groups");
        expect(sheet.openName).toBe("groups");

        drag(
            el.querySelector(".rad-sheet__handle") as Element,
            100,
            100 + CoverDrag.DISMISS + 1,
        );
        expect(sheet.openName).toBeNull();
    });

    // Short of it the sheet springs back, so a hesitant touch does not lose the list.
    it("stays open on a drag that stops short", () => {
        const el = frame();
        const sheet = new Sheet(el);
        sheet.open("groups");

        drag(el.querySelector(".rad-sheet__handle") as Element, 100, 130);
        expect(sheet.openName).toBe("groups");
        expect((el.querySelector("[data-rad-sheet]") as HTMLElement).style.transform).toBe("");
    });

    // Otherwise picking a group would be a drag on the sheet holding it.
    it("does not claim a press on a row", () => {
        const el = frame();
        const sheet = new Sheet(el);
        sheet.open("groups");

        drag(el.querySelector(".rad-list-row") as Element, 100, 100 + CoverDrag.DISMISS + 1);
        expect(sheet.openName).toBe("groups");
    });

    // A closed sheet is off screen; a stray pointer on it must not reopen or move anything.
    it("ignores a drag on a sheet that is not open", () => {
        const el = frame();
        const sheet = new Sheet(el);

        drag(
            el.querySelector(".rad-sheet__handle") as Element,
            100,
            100 + CoverDrag.DISMISS + 1,
        );
        expect(sheet.openName).toBeNull();
        expect((el.querySelector("[data-rad-sheet]") as HTMLElement).style.transform).toBe("");
    });

    // Closing leaves the sheet ready to open again from its resting position.
    it("clears the drag transform when it closes", () => {
        const el = frame();
        const sheet = new Sheet(el);
        sheet.open("groups");
        const node = el.querySelector("[data-rad-sheet]") as HTMLElement;

        node.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, clientY: 100 }));
        node.dispatchEvent(new PointerEvent("pointermove", { bubbles: true, clientY: 240 }));
        expect(node.style.transform).not.toBe("");

        node.dispatchEvent(new PointerEvent("pointerup", { bubbles: true, clientY: 240 }));
        expect(node.style.transform).toBe("");
        expect(node.classList.contains("is-dragging")).toBe(false);
        expect(sheet.openName).toBeNull();
    });
});
