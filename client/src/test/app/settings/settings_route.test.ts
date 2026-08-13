import { describe, expect, it } from "vitest";
import { SettingsRoute } from "../../../js/app/settings/SettingsRoute";

describe("SettingsRoute", () => {
    it("puts a pane under the dashboard, so the dashboard survives the navigation", () => {
        expect(SettingsRoute.href("audio")).toBe("/dashboard/settings/audio");
    });

    it("reads the pane back out of a path", () => {
        expect(SettingsRoute.paneOf("/dashboard/settings/connect")).toBe("connect");
    });

    it("tolerates a trailing slash, which is what a prerendered route serves", () => {
        expect(SettingsRoute.paneOf("/dashboard/settings/connect/")).toBe("connect");
    });

    it("falls back when the path names no pane", () => {
        expect(SettingsRoute.paneOf("/dashboard/settings")).toBe("account");
        expect(SettingsRoute.paneOf("/dashboard")).toBe("account");
    });

    // A link kept from a desktop session, opened on a phone. Landing on the fallback is
    // recoverable; landing on a titled empty stage is not.
    it("falls back for a pane this build does not have", () => {
        expect(SettingsRoute.paneOf("/dashboard/settings/recordings", true)).toBe("account");
        expect(SettingsRoute.paneOf("/dashboard/settings/recordings", false)).toBe("recordings");
    });

    it("falls back for a pane that never existed", () => {
        expect(SettingsRoute.paneOf("/dashboard/settings/nonsense")).toBe("account");
    });
});
