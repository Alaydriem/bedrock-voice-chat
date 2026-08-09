import { PlayerLevelSources } from "../../js/app/dashboard/PlayerLevelSources";
import { render } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { mockInvoke } from "../tauri";

vi.mock("@tauri-apps/api/webviewWindow", () => ({
    getCurrentWebviewWindow: () => ({ listen: async () => () => {} }),
}));

const { default: DashboardScreen } = await import(
    "../../components/dashboard/DashboardScreen.svelte"
);
const { SelfController } = await import("../../js/app/dashboard/SelfController");

function controller() {
    mockInvoke({ mute_status: () => false, is_recording: () => false, set_mute: () => true });
    return new SelfController({ get: async () => undefined } as never, new PlayerLevelSources());
}

/**
 * The frame, built by hand.
 *
 * `RadFrame` is what supplies it in the app, and these are assertions about where things land
 * relative to it — so the test provides one and mounts into it, exactly as the real page does.
 */
function mount(props: Record<string, unknown> = {}) {
    const stage = document.createElement("div");
    const frame = document.createElement("div");
    frame.className = "rad-frame rad-frame--fluid";
    stage.append(frame);
    document.body.append(stage);

    const rendered = render(DashboardScreen, {
        target: frame,
        props: {
            servers: [{ server: "https://a.example.com", host: "a.example.com", player: "Al", isCurrent: true }],
            serverName: "a.example.com",
            currentHost: "a.example.com",
            player: "Alaydriem",
            self: controller(),
            selfState: {
                muted: false,
                deafened: false,
                recording: false,
                mode: "activated",
                holding: false,
                transmitting: true,
                recordAllowed: true,
            },
            headline: "NOBODY IN EARSHOT",
            onswitch: () => {},
            onadd: () => {},
            onsettings: () => {},
            onsignout: () => {},
            onstatus: () => {},
            main: undefined,
            ...props,
        } as never,
    });

    return { frame, rendered };
}

describe("DashboardScreen structure", () => {
    beforeEach(() => {
        document.body.innerHTML = "";
    });

    /**
     * The bug this exists for.
     *
     * Wrapping these in a `.rad-screen` made every one of them measure against a flex container
     * that clips horizontally and not vertically, so the sheets — `bottom: 0` under a
     * `translateY(102%)` — hung below the fold as scrollable content instead of waiting
     * off-screen. Nothing about it fails to compile, and no unit test on a manager can see it.
     */
    it("puts its overlays on the frame rather than inside a screen", () => {
        const { frame } = mount();

        expect(frame.querySelector(".rad-screen")).toBeNull();
        for (const selector of [".rad-menu", ".rad-scrim", ".rad-self-pill"]) {
            const el = frame.querySelector(selector);
            expect(el, `${selector} is missing`).not.toBeNull();
            expect(el!.closest(".rad-frame")).toBe(frame);
            expect(el!.parentElement).toBe(frame);
        }
    });

    // The groups button used to open nothing, because only the servers sheet existed.
    it("has a sheet for every sheet a button opens", () => {
        const { frame } = mount({ groups: undefined });

        const opened = [...frame.querySelectorAll<HTMLElement>("[data-rad-sheet-open]")].map(
            (el) => el.dataset.radSheetOpen,
        );
        expect(opened).toContain("servers");
        expect(opened).toContain("groups");

        for (const name of opened) {
            expect(
                frame.querySelector(`[data-rad-sheet="${name}"]`),
                `no sheet named ${name}`,
            ).not.toBeNull();
        }
    });

    it("keeps every sheet a direct child of the frame, where its offset is measured from", () => {
        const { frame } = mount();

        for (const sheet of frame.querySelectorAll(".rad-sheet")) {
            expect(sheet.parentElement).toBe(frame);
        }
    });
});

describe("DashboardScreen session menu", () => {
    beforeEach(() => {
        document.body.innerHTML = "";
    });

    /**
     * One set of actions, whichever edge of the screen you came in from.
     *
     * The chevron beside your name, the server glyph in the corner and the slide-up sheet were
     * three separately-written lists: the chevron offered "Switch server" and no status, the
     * sheet offered status and no identity. Nobody can learn a menu that changes depending on
     * which control opened it, and nothing but an assertion keeps three call sites in step.
     */
    it("offers the same actions in the sheet as the menu", async () => {
        const { frame } = mount();

        const sheet = frame.querySelector<HTMLElement>('[data-rad-sheet="servers"]')!;
        const rows = [...sheet.querySelectorAll(".rad-list-row")].map((el) =>
            el.textContent?.trim(),
        );
        expect(rows).toEqual(["Add a server", "Settings", "Connection status", "Sign out"]);

        // A wide frame takes the dropdown path; `clientWidth` is zero in jsdom, which is the
        // phone branch, so the desktop menu is driven by widening the frame first.
        Object.defineProperty(frame, "clientWidth", { value: 1200, configurable: true });
        frame.querySelector<HTMLButtonElement>(".rad-self-pill .rad-self__id")!.click();
        await Promise.resolve();

        const items = [...frame.querySelectorAll(".rad-menu__item")].map((el) =>
            el.textContent?.replace("signed in", "").trim(),
        );
        for (const label of rows) {
            expect(items, `the menu is missing ${label}`).toContain(label);
        }
    });

    // Anchored to the pill's chevron. Both the pill and the phone capsule render a
    // `.rad-self__id`, and the capsule comes first in document order — an unscoped lookup
    // anchored the menu to a `display: none` element, whose zero rect clamps it to the corner.
    it("anchors the menu to the pill rather than the hidden capsule", async () => {
        const { frame } = mount();
        Object.defineProperty(frame, "clientWidth", { value: 1200, configurable: true });

        const chevron = frame.querySelector<HTMLButtonElement>(".rad-self-pill .rad-self__id")!;
        chevron.click();
        await Promise.resolve();

        expect(chevron.getAttribute("aria-expanded")).toBe("true");
        expect(
            frame.querySelector<HTMLElement>(".rad-self-capsule .rad-self__id")
                ?.getAttribute("aria-expanded"),
        ).not.toBe("true");
    });

    // The phone has no room for a dropdown anchored to a control at the bottom of the screen: it
    // flips upward into the roster and lands under the thumb that opened it.
    it("sends a phone-width frame to the sheet instead of the dropdown", async () => {
        const { frame } = mount();
        Object.defineProperty(frame, "clientWidth", { value: 420, configurable: true });

        frame.querySelector<HTMLButtonElement>(".rad-self-pill .rad-self__id")!.click();
        await Promise.resolve();

        expect(frame.querySelector(".rad-menu")?.classList.contains("is-open")).toBe(false);
        expect(
            frame.querySelector('[data-rad-sheet="servers"]')?.classList.contains("is-open"),
        ).toBe(true);
    });
});

describe("DashboardScreen frame state", () => {
    beforeEach(() => {
        document.body.innerHTML = "";
    });

    /**
     * The coloured stripe across the top of the stage is `.rad-frame.is-muted .rad-stage::before`.
     * A muted mic is a property of the session rather than of the button that set it, so the kit
     * draws it on the frame — and toggling only the button's icon leaves it permanently absent.
     */
    it("marks the frame muted, which is what draws the stripe", async () => {
        const { frame, rendered } = mount();
        expect(frame.classList.contains("is-muted")).toBe(false);

        await rendered.rerender({
            selfState: {
                muted: true,
                deafened: false,
                recording: false,
                mode: "activated",
                holding: false,
                transmitting: false,
                recordAllowed: true,
            },
        } as never);

        expect(frame.classList.contains("is-muted")).toBe(true);
    });

    // Deafen has its own colour, and it implies mute — so the muted stripe must not also be on,
    // or the two backgrounds fight over the same three pixels.
    it("marks the frame deafened instead of muted when both are set", async () => {
        const { frame, rendered } = mount();

        await rendered.rerender({
            selfState: {
                muted: true,
                deafened: true,
                recording: false,
                mode: "activated",
                holding: false,
                transmitting: false,
                recordAllowed: true,
            },
        } as never);

        expect(frame.classList.contains("is-deafened")).toBe(true);
        expect(frame.classList.contains("is-muted")).toBe(false);
    });

    // `.rad-status` is `opacity: 0; pointer-events: none` until the frame says otherwise, so
    // mounting the panel without this class shows nothing at all.
    it("marks the frame is-status so the panel can be seen", async () => {
        const { frame, rendered } = mount();
        expect(frame.classList.contains("is-status")).toBe(false);

        await rendered.rerender({ statusOpen: true } as never);

        expect(frame.classList.contains("is-status")).toBe(true);
    });
});

describe("DashboardScreen groups sheet", () => {
    /**
     * The panel grows while a group is being renamed. Anchored to the bottom, that growth
     * pushes every row upward — the row being edited moves out from under the eye — so the
     * sheet opens to the top of the frame instead.
     */
    it("opens the groups sheet to the top of the frame", () => {
        const { frame } = mount();
        const sheet = frame.querySelector('[data-rad-sheet="groups"]');
        expect(sheet?.classList.contains("rad-sheet--full")).toBe(true);
    });

    // A full-height sheet has to scroll inside itself. Without a body the whole sheet
    // scrolls, which takes the handle off the top of it.
    it("gives the groups sheet a body to scroll", () => {
        const { frame } = mount();
        const sheet = frame.querySelector('[data-rad-sheet="groups"]');
        expect(sheet?.querySelector(".rad-sheet__body")).not.toBeNull();
        expect(sheet?.querySelector(".rad-sheet__handle")).not.toBeNull();
    });

    // The servers sheet is a fixed list of actions, so it stays where it was.
    it("leaves the servers sheet at the bottom", () => {
        const { frame } = mount();
        const sheet = frame.querySelector('[data-rad-sheet="servers"]');
        expect(sheet?.classList.contains("rad-sheet--full")).toBe(false);
    });
});

describe("DashboardScreen chat", () => {
    beforeEach(() => {
        document.body.innerHTML = "";
    });

    /**
     * The dock itself is `ChatDock`'s concern and arrives as a snippet, so this asserts the
     * seam rather than the contents: the screen owns the phone tabs, and renders whatever chat
     * surface it is handed.
     *
     * Chat used to be present and inert here. It is now live on the no-net Bedrock path, so
     * the old assertion that the input is disabled would encode a state the product left.
     */
    it("owns the phone tabs that make chat a peer view of the roster", () => {
        const { frame } = mount();

        const tabs = [...frame.querySelectorAll(".rad-tabs button")].map((b) =>
            b.textContent?.trim(),
        );
        expect(tabs).toEqual(["In earshot", "Chat"]);
    });

    it("renders no chat surface when none is supplied", () => {
        const { frame } = mount();

        expect(frame.querySelector(".rad-chat-dock")).toBeNull();
    });

    /**
     * The kit collapses `.rad-chat-history` to zero height until the frame carries `is-chat`.
     *
     * Missing this shipped a dock whose composer worked and whose scrollback was invisible:
     * every received line rendered into a box of no height, which reads as "chat is broken"
     * rather than "a class is missing".
     */
    it("gives the frame is-chat so the scrollback has a height", () => {
        const { frame } = mount({ chatOpen: true });

        expect(frame.classList.contains("is-chat")).toBe(true);
    });

    it("leaves is-chat off while the scrollback is shut", () => {
        const { frame } = mount({ chatOpen: false });

        expect(frame.classList.contains("is-chat")).toBe(false);
    });
});
