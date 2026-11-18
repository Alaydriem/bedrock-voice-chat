import { afterEach, describe, expect, it, vi } from "vitest";
import { render } from "@testing-library/svelte";

vi.mock("../../../components/settings/SettingsScreen.svelte", async () => ({
    default: (await import("../../stubs/Empty.svelte")).default,
}));
vi.mock("../../../components/events/Notification.svelte", async () => ({
    default: (await import("../../stubs/Empty.svelte")).default,
}));

import SettingsPage from "../../../routes/settings/+page.svelte";

describe("standalone settings route", () => {
    afterEach(() => {
        vi.useRealTimers();
        document.querySelector(".app-preloader")?.remove();
    });

    // The error screen's "Change Audio Devices" arrives here with a full document load.
    // An overlay left up hides a working settings screen behind "Connecting…".
    it("takes down the boot overlay", () => {
        vi.useFakeTimers();
        const overlay = document.createElement("div");
        overlay.className = "app-preloader";
        document.body.appendChild(overlay);

        render(SettingsPage);
        vi.advanceTimersByTime(1000);

        expect(document.querySelector(".app-preloader")).toBeNull();
    });
});
