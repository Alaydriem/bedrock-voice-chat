import { render } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../tauri";

let platformName = "windows";
vi.mock("@tauri-apps/plugin-os", () => ({ platform: () => platformName }));

const { default: Harness } = await import("./RadFrameHarness.svelte");

function mount(): HTMLElement {
    const host = document.createElement("div");
    document.body.append(host);
    render(Harness as never, { target: host } as never);
    return host.querySelector(".rad-frame") as HTMLElement;
}

describe("the frame's platform class", () => {
    beforeEach(() => {
        platformName = "windows";
    });

    it("leaves a desktop frame unmarked, whatever width the window is dragged to", () => {
        expect(mount().classList.contains("rad-frame--mobile")).toBe(false);
    });

    // Every other responsive decision in the kit is a container query, and a container query
    // cannot tell an iPad from a desktop window of the same width. A tablet in landscape is
    // wider than every breakpoint the kit has, so it takes the desktop branch of all of them.
    it("marks the frame on iOS, which is what an iPad reports", () => {
        platformName = "ios";

        expect(mount().classList.contains("rad-frame--mobile")).toBe(true);
    });

    it("marks the frame on Android", () => {
        platformName = "android";

        expect(mount().classList.contains("rad-frame--mobile")).toBe(true);
    });
});
