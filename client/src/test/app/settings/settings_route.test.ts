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

    // Path-to-pane resolution cannot know the viewer's permissions: they load
    // asynchronously from the keyring, and this is a synchronous derivation off the URL.
    // Gating here sent every navigation to a permission-gated pane to the fallback, which
    // reads as a sidebar item that does nothing. The gate belongs in SettingsScreen, where
    // the permissions are known.
    it("resolves a permission-gated pane, because the route layer holds no permissions", () => {
        expect(SettingsRoute.paneOf("/dashboard/settings/manage-players")).toBe("manage-players");
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

describe("SettingsRoute.isPane", () => {
    it("separates a named pane from the section list", () => {
        expect(SettingsRoute.isPane("/dashboard/settings/audio")).toBe(true);
        expect(SettingsRoute.isPane("/dashboard/settings")).toBe(false);
    });

    // A trailing slash is the section list with punctuation, not a pane named "".
    it("treats a trailing slash as the section list", () => {
        expect(SettingsRoute.isPane("/dashboard/settings/")).toBe(false);
    });

    it("claims nothing outside settings", () => {
        expect(SettingsRoute.isPane("/dashboard")).toBe(false);
        expect(SettingsRoute.isPane("/login")).toBe(false);
    });
});

/**
 * Depth is how many entries sit between this screen and the dashboard. It is the number
 * of pops that leaves settings, so it is what `exit` counts with.
 */
describe("SettingsRoute.depthOf", () => {
    // A phone shows the section list and the pane as two screens.
    it("counts the list and the pane separately on mobile", () => {
        expect(SettingsRoute.depthOf("/dashboard/settings", true)).toBe(1);
        expect(SettingsRoute.depthOf("/dashboard/settings/audio", true)).toBe(2);
    });

    // Desktop shows the section nav and the pane side by side: one screen, one level.
    it("collapses the list and the pane on desktop", () => {
        expect(SettingsRoute.depthOf("/dashboard/settings/audio", false)).toBe(1);
    });

    it("puts the dashboard at the bottom", () => {
        expect(SettingsRoute.depthOf("/dashboard", true)).toBe(0);
        expect(SettingsRoute.depthOf("/dashboard", false)).toBe(0);
    });
});

describe("SettingsRoute.parentOf", () => {
    it("climbs a mobile pane to the section list", () => {
        expect(SettingsRoute.parentOf("/dashboard/settings/audio", true)).toBe(
            "/dashboard/settings",
        );
    });

    it("climbs a desktop pane straight to the dashboard", () => {
        expect(SettingsRoute.parentOf("/dashboard/settings/audio", false)).toBe("/dashboard");
    });

    it("climbs the section list to the dashboard", () => {
        expect(SettingsRoute.parentOf("/dashboard/settings", true)).toBe("/dashboard");
    });

    // Nothing is above the dashboard. Back there is Android's to handle, not ours.
    it("keeps the dashboard as its own ceiling", () => {
        expect(SettingsRoute.parentOf("/dashboard", true)).toBe("/dashboard");
    });
});
