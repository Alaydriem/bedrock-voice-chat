import { describe, expect, it } from "vitest";
import { SettingsCatalogue } from "../../../js/app/settings/SettingsCatalogue";

describe("SettingsCatalogue", () => {
    it("opens on the account pane", () => {
        expect(SettingsCatalogue.fallback).toBe("account");
        expect(SettingsCatalogue.find("account", false)).not.toBeNull();
    });

    // Recording is not supported on mobile and there is no global shortcut to bind on a
    // phone. A pane that exists only to explain its own absence is a dead end in a list
    // somebody scans every time they open settings.
    it("drops the panes the mobile build does not have", () => {
        const ids = SettingsCatalogue.for(true).map((pane) => pane.id);
        expect(ids).not.toContain("recordings");
        expect(ids).not.toContain("keybinds");
        expect(ids).toContain("audio");
        expect(ids).toContain("library");
    });

    it("keeps every pane on desktop", () => {
        expect(SettingsCatalogue.for(false)).toHaveLength(9);
    });

    // A deep link to a pane this build does not have must not resolve, or the router
    // lands on a blank stage with a title above it.
    it("refuses to find a desktop-only pane on mobile", () => {
        expect(SettingsCatalogue.find("recordings", true)).toBeNull();
        expect(SettingsCatalogue.find("recordings", false)?.title).toBe("Recordings");
    });

    it("refuses to find a pane that does not exist at all", () => {
        expect(SettingsCatalogue.find("nonsense", false)).toBeNull();
    });

    it("has no empty groups on mobile", () => {
        for (const group of SettingsCatalogue.groups(true)) {
            expect(group.panes.length).toBeGreaterThan(0);
        }
    });

    it("keeps the Bedrock group on both builds", () => {
        for (const mobile of [true, false]) {
            const names = SettingsCatalogue.groups(mobile).map((group) => group.name);
            expect(names).toContain("Minecraft Bedrock");
        }
    });

    // Plates and a seven-column table both need more than the 760px row measure.
    it("marks the panes that need the wide measure", () => {
        const wide = SettingsCatalogue.for(false)
            .filter((pane) => pane.wide)
            .map((pane) => pane.id);
        expect(wide.sort()).toEqual(["connect", "library", "players"]);
    });
    // The roster, the whitelist and the ban button are the same permission the server
    // checks. A viewer without it must not see the pane at all — an empty pane that 403s
    // on open is worse than no pane.
    it("hides the admin pane from a viewer with no permissions", () => {
        const ids = SettingsCatalogue.for(false).map((pane) => pane.id);
        expect(ids).not.toContain("manage-players");
        expect(SettingsCatalogue.for(false)).toHaveLength(9);
    });

    it("shows the admin pane to a viewer holding admin", () => {
        const ids = SettingsCatalogue.for(false, ["admin"]).map((pane) => pane.id);
        expect(ids).toContain("manage-players");
        expect(SettingsCatalogue.for(false, ["admin"])).toHaveLength(10);
    });

    // Another permission is not this permission.
    it("does not accept audio_upload as admin", () => {
        const ids = SettingsCatalogue.for(false, ["audio_upload"]).map((pane) => pane.id);
        expect(ids).not.toContain("manage-players");
    });

    // `find` is the gated lookup: the sidebar builds from it and SettingsScreen renders
    // from it. A viewer without the permission therefore gets the fallback pane rendered,
    // which is where a deep link is actually stopped.
    it("refuses to find the admin pane without the permission", () => {
        expect(SettingsCatalogue.find("manage-players", false)).toBeNull();
        expect(SettingsCatalogue.find("manage-players", false, ["admin"])?.title).toBe(
            "Manage Players",
        );
    });

    it("keeps the admin pane on the mobile build", () => {
        const ids = SettingsCatalogue.for(true, ["admin"]).map((pane) => pane.id);
        expect(ids).toContain("manage-players");
    });

    it("groups the admin pane under Server, and only for an admin", () => {
        expect(SettingsCatalogue.groups(false).map((group) => group.name)).not.toContain("Server");
        expect(SettingsCatalogue.groups(false, ["admin"]).map((group) => group.name)).toContain(
            "Server",
        );
    });

    // `resolve` is what the router uses. It applies the platform filter and deliberately
    // not the permission gate, because a path is resolved synchronously and the permission
    // set is not known until the keyring answers.
    it("resolves a permission-gated pane for the router", () => {
        expect(SettingsCatalogue.resolve("manage-players", false)?.id).toBe("manage-players");
    });

    it("still keeps a desktop-only pane away from the router on mobile", () => {
        expect(SettingsCatalogue.resolve("recordings", true)).toBeNull();
        expect(SettingsCatalogue.resolve("recordings", false)?.id).toBe("recordings");
    });

    it("resolves nothing for a pane that does not exist", () => {
        expect(SettingsCatalogue.resolve("nonsense", false)).toBeNull();
    });

    // A seven-column roster does not fit the row measure.
    it("marks the admin pane as wide", () => {
        expect(SettingsCatalogue.find("manage-players", false, ["admin"])?.wide).toBe(true);
    });

    // The pane beside it now manages the roster, so "Players" no longer says which is which.
    it("names the per-player audio pane by what it changes", () => {
        expect(SettingsCatalogue.find("players", false)?.title).toBe("Player audio levels");
    });
});
