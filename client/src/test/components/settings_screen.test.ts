import { render, waitFor } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";

const onnavigate = vi.fn();
const onclose = vi.fn();
const onback = vi.fn();

let platformName = "windows";
vi.mock("@tauri-apps/plugin-os", () => ({ platform: () => platformName }));

const { default: SettingsScreen } = await import(
    "../../components/settings/SettingsScreen.svelte"
);
const { UpdateStatus } = await import("../../js/app/settings/UpdateStatus");

/** An `UpdateStatus` already settled on a verdict, so the badge is not racing a check. */
async function settled(version: string | null) {
    const updates = new UpdateStatus(async () => version);
    await updates.check();
    return updates;
}

function mount(props: Record<string, unknown> = {}) {
    const frame = document.createElement("div");
    frame.className = "rad-frame rad-frame--fluid";
    document.body.append(frame);

    render(SettingsScreen, {
        target: frame,
        props: { pane: "account", level: "detail", onnavigate, onclose, onback, ...props },
    });

    return {
        frame,
        shell: () => frame.querySelector<HTMLElement>(".rad-settings"),
        measure: () => frame.querySelector<HTMLElement>(".rad-settings-measure"),
        navItems: () => [...frame.querySelectorAll<HTMLElement>(".rad-panel__body .rad-nav-item")],
        groupHeads: () => [...frame.querySelectorAll<HTMLElement>(".rad-nav-group")],
        title: () => frame.querySelector<HTMLElement>(".rad-dash-top__server")?.textContent?.trim(),
        backIcon: () =>
            frame.querySelector<HTMLElement>(".rad-backbar__btn [data-rad-icon]")?.dataset.radIcon,
        backButton: () => frame.querySelector<HTMLElement>(".rad-backbar__btn"),
    };
}

beforeEach(() => {
    onnavigate.mockClear();
    onclose.mockClear();
    onback.mockClear();
    platformName = "windows";
});

describe("SettingsScreen", () => {
    it("lists every pane on desktop", async () => {
        const view = mount();
        await waitFor(() => expect(view.navItems()).toHaveLength(9));
        expect(view.groupHeads().map((el) => el.textContent)).toContain("Minecraft Bedrock");
    });

    // Recording is not supported on mobile and there is no global shortcut to bind on a
    // phone. Capability follows the platform, never the width.
    it("drops the panes the mobile build does not have", async () => {
        platformName = "android";
        const view = mount();
        await waitFor(() => expect(view.navItems()).toHaveLength(7));
        const labels = view.navItems().map((el) => el.textContent?.trim());
        expect(labels).not.toContain("Recordings");
        expect(labels).not.toContain("Keybinds");
        // The WebSocket server is not one of them. A phone can bind a listening socket and
        // keeps the process alive through the audio foreground service, so the pane belongs
        // on both builds.
        expect(labels).toContain("WebSocket server");
    });

    it("marks the pane it is showing", async () => {
        const view = mount({ pane: "audio" });
        await waitFor(() => {
            const on = view.navItems().find((el) => el.getAttribute("aria-current") === "page");
            expect(on?.textContent?.trim()).toBe("Audio settings");
        });
        expect(view.title()).toBe("Audio settings");
    });

    // Plates and a seven-column table both need more than the 760px row measure.
    it("widens the measure only for the panes that need it", async () => {
        expect(mount({ pane: "connect" }).measure()?.classList.contains("is-wide")).toBe(true);
        expect(mount({ pane: "account" }).measure()?.classList.contains("is-wide")).toBe(false);
    });

    it("navigates to a pane rather than swapping it in place", async () => {
        const view = mount();
        await waitFor(() => expect(view.navItems()).toHaveLength(9));
        view.navItems().find((el) => el.textContent?.includes("Keybinds"))?.click();
        expect(onnavigate).toHaveBeenCalledWith("keybinds");
    });

    // At the section list back leaves settings, so it is an X: an arrow would promise a
    // screen that is not there.
    it("shows an X at the section list and an arrow inside a pane", async () => {
        expect(mount({ level: "list" }).backIcon()).toBe("close");
        expect(mount({ level: "detail" }).backIcon()).toBe("back");
    });

    // Where back goes is the route's decision, so the button reports the press and does
    // not act on it. One callback at both levels is what keeps the on-screen button and
    // the platform back from ever disagreeing.
    it("reports the back press at either level without choosing a destination", async () => {
        const pane = mount({ level: "detail" });
        pane.backButton()?.click();
        expect(onback).toHaveBeenCalledTimes(1);
        expect(onclose).not.toHaveBeenCalled();

        onback.mockClear();
        const list = mount({ level: "list" });
        list.backButton()?.click();
        expect(onback).toHaveBeenCalledTimes(1);
        expect(onclose).not.toHaveBeenCalled();
    });

    // The gear in the rail and the X in the header leave settings outright from any
    // level, so they are the one control that stays wired to `onclose`.
    it("leaves outright from the rail gear", async () => {
        const view = mount({ level: "detail" });
        await waitFor(() => expect(view.navItems()).toHaveLength(9));
        view.frame.querySelector<HTMLElement>(".rad-rail-btn")?.click();
        expect(onclose).toHaveBeenCalledTimes(1);
    });

    // Driven through the injected UpdateStatus rather than a flag, because that object is
    // what the shell polls and what the About row reads — a flag would have proved the badge
    // renders without proving anything reaches it.
    it("badges About only when an update is waiting", async () => {
        const waiting = mount({ updates: await settled("1.0.0-beta.9") });
        await waitFor(() =>
            expect(waiting.frame.querySelector(".rad-nav-item__badge")).not.toBeNull(),
        );

        const current = mount({ updates: await settled(null) });
        expect(current.frame.querySelector(".rad-nav-item__badge")).toBeNull();
    });
});
